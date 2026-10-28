#![cfg(unix)]
mod support;
use lapis_ai_service::{AcpHost, Config, HostContext, HostPolicy, Launch, Result, Target};
use std::{sync::Arc, time::Duration};

struct PacedPolicy(Arc<support::Policy>);
impl HostPolicy for PacedPolicy {
    fn authorize(&self, context: &HostContext) -> Result<String> {
        self.0.authorize(context)
    }
    fn targets(&self, context: &HostContext) -> Result<Vec<Target>> {
        self.0.targets(context)
    }
    fn resolve(&self, context: &HostContext, target: &str, resume: Option<&str>) -> Result<Launch> {
        let launch = self.0.resolve(context, target, resume)?;
        std::fs::write(&launch.prepared.executable, PEER).unwrap();
        Ok(launch)
    }
}

#[tokio::test]
async fn paced_output_preserves_all_events_with_bounded_channel_delivery_cadence() {
    let root = tempfile::tempdir().unwrap();
    let (_, policy) = support::setup(root.path());
    let host = Arc::new(AcpHost::new(Arc::new(PacedPolicy(policy)), Config::default()).unwrap());
    let context = support::context("one");
    let info = host.open(&context, "open", "fixture", None).await.unwrap();
    support::configure(&host, &context, &info.session_id).await;
    let (send, mut receive) = tokio::sync::mpsc::unbounded_channel();
    let token = host
        .attach(
            &context,
            &info.session_id,
            0,
            Arc::new(move |frame| send.send(frame).map_err(|_| ())),
        )
        .unwrap();
    host.submit(&context, &info.session_id, "turn".into(), support::prompt())
        .await
        .unwrap();
    let (deliveries, text, cursors) = tokio::time::timeout(Duration::from_secs(5), async {
        let mut deliveries = 0;
        let mut text = Vec::new();
        let mut cursors = Vec::new();
        while let Some(message) = receive.recv().await {
            deliveries += 1;
            assert!(message.snapshot.failure.is_none());
            let mut complete = false;
            for event in &message.snapshot.events {
                cursors.push(event.sequence);
                for frame in event.frames() {
                    if frame["type"] == "activity" {
                        text.push(
                            frame["update"]["content"]["text"]
                                .as_str()
                                .unwrap()
                                .to_owned(),
                        );
                    }
                    complete |= frame["type"] == "record";
                }
            }
            host.acknowledge(
                &context,
                &info.session_id,
                &token,
                message.snapshot.sequence,
            )
            .unwrap();
            if complete {
                break;
            }
        }
        (deliveries, text, cursors)
    })
    .await
    .unwrap();
    host.shutdown().await.unwrap();
    assert_eq!(
        text,
        (0..200)
            .map(|number| number.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(cursors, (1..=cursors.len() as u64).collect::<Vec<_>>());
    // Require at least 2× coalescing. An absolute delivery cap is wall-clock
    // sensitive under loaded CI hosts even when the 16ms cadence holds.
    assert!(
        deliveries * 2 < text.len(),
        "200 paced updates caused {deliveries} native channel deliveries"
    );
    assert!(support::all_stopped(root.path(), "one"));
}

#[tokio::test]
async fn admission_revocation_fences_frames_waiting_for_channel_delivery() {
    let root = tempfile::tempdir().unwrap();
    let (_, policy) = support::setup(root.path());
    let host =
        Arc::new(AcpHost::new(Arc::new(PacedPolicy(policy.clone())), Config::default()).unwrap());
    let context = support::context("one");
    let info = host.open(&context, "open", "fixture", None).await.unwrap();
    support::configure(&host, &context, &info.session_id).await;
    let (send, mut receive) = tokio::sync::mpsc::unbounded_channel();
    let token = host
        .attach(
            &context,
            &info.session_id,
            0,
            Arc::new(move |frame| send.send(frame).map_err(|_| ())),
        )
        .unwrap();
    let first = receive.recv().await.unwrap();
    let after = first.snapshot.sequence;
    host.acknowledge(&context, &info.session_id, &token, after)
        .unwrap();
    host.submit(&context, &info.session_id, "turn".into(), support::prompt())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while host
            .snapshot(&context, &info.session_id, after)
            .unwrap()
            .sequence
            == after
        {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    policy
        .generation
        .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    let late = tokio::time::timeout(Duration::from_millis(60), receive.recv()).await;
    host.shutdown().await.unwrap();
    assert!(
        !matches!(late, Ok(Some(_))),
        "Revoked frames must not reach the channel"
    );
    assert!(support::all_stopped(root.path(), "one"));
}

const PEER: &str = r#"#!/usr/bin/env python3
import sys,os,json,time
log=open(os.environ['LOG'],'a',buffering=1)
log.write(json.dumps({'pid':os.getpid()})+'\n')
scope=None
sequence=0
def emit(kind,rid,**body):
 global sequence
 sequence+=1
 frame={'version':3,'type':kind,'requestId':rid,'sequence':sequence,**body}
 if scope:frame['sessionId']=scope
 print(json.dumps(frame),flush=True)
print(json.dumps({'version':3,'type':'ready'}),flush=True)
for line in sys.stdin:
 f=json.loads(line);log.write(json.dumps(f)+'\n');rid=f['requestId'];kind=f['type']
 if kind=='warm':emit('warmed',rid)
 elif kind=='open':scope=f['sessionId'];emit('opened',rid,adapterSessionId='provider',session={})
 elif kind=='configure':emit('configured',rid,modeReady=True)
 elif kind=='prompt':
  emit('started',rid,adapterSessionId='provider')
  for i in range(200):
   emit('activity',rid,update={'content':{'text':str(i)}})
   time.sleep(0.001)
  emit('record',rid,record={'version':1,'taskId':rid,'status':'completed'},sessionUsable=True,quiescent=True)
"#;
