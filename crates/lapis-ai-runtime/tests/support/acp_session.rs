#![allow(dead_code)]
use lapis_ai_runtime::{
    AcpSession, Environment, PreparedLaunch, SessionOptions, SessionRequest, SessionSnapshot,
};
use serde_json::Value;
use std::{os::unix::fs::PermissionsExt, path::Path, time::Duration};

pub fn plan(root: &Path) -> PreparedLaunch {
    let helper = root.join("peer.py");
    std::fs::write(&helper, PEER).unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut environment = Environment::default();
    environment
        .0
        .insert("PATH".into(), std::env::var("PATH").unwrap());
    environment.0.insert(
        "LOG".into(),
        root.join("frames.jsonl").to_string_lossy().into_owned(),
    );
    PreparedLaunch {
        executable: helper,
        arguments: vec!["unsafe-override".into()],
        directory: root.into(),
        environment,
    }
}
pub fn options(root: &Path, id: &str) -> SessionOptions {
    let mut options = SessionOptions::new(id.into(), root.into());
    options.request_timeout = Duration::from_millis(300);
    options
}
pub async fn start(root: &Path, id: &str) -> AcpSession {
    AcpSession::start(plan(root), options(root, id))
        .await
        .unwrap()
}
pub fn prompt(text: &str) -> SessionRequest {
    SessionRequest::Prompt {
        text: text.into(),
        context: Some("private context".into()),
        max_duration_ms: 150,
    }
}
pub async fn configured(session: &AcpSession) {
    session
        .submit(
            "configure".into(),
            SessionRequest::Configure {
                model: Some("fixture-model".into()),
                effort: Some("high".into()),
                mode: Some("ask".into()),
            },
        )
        .await
        .unwrap();
    wait(session, |snapshot| {
        snapshot.events.iter().any(|event| {
            event.frame["type"] == "configured" && event.frame["requestId"] == "configure"
        })
    })
    .await;
}
pub async fn wait(
    session: &AcpSession,
    predicate: impl Fn(&SessionSnapshot) -> bool,
) -> SessionSnapshot {
    let mut changed = session.subscribe().unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = session.snapshot(0).unwrap();
            if predicate(&snapshot) {
                return snapshot;
            }
            assert!(
                !snapshot.closed,
                "worker closed: {:?}; unresolved: {:?}",
                snapshot.failure, snapshot.unresolved_requests
            );
            changed.changed().await.unwrap();
        }
    })
    .await
    .unwrap()
}
pub fn frames(root: &Path) -> Vec<Value> {
    std::fs::read_to_string(root.join("frames.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
pub fn stopped(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_err()
}
pub async fn reaped(pid: u32) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !stopped(pid) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
const PEER: &str = r#"#!/usr/bin/env python3
import os,sys,json
log=open(os.environ['LOG'],'a',buffering=1)
log.write(json.dumps({'pid':os.getpid(),'args':sys.argv[1:],'root':os.getcwd(),'launchRoot':os.environ.get('JJ_ACP_CWD')})+'\n')
seq=0
scope=None
active=None
def emit(kind,request_id,**body):
 global seq
 seq+=1
 value={'version':3,'type':kind,'requestId':request_id,'sequence':seq,**body}
 if scope is not None and kind!='status':value['sessionId']=scope
 value.update(body)
 print(json.dumps(value),flush=True)
def record(request,status='completed'):
 emit('record',request,record={'version':1,'taskId':request,'status':status},sessionUsable=True,quiescent=True)
print(json.dumps({'version':3,'type':'ready'}),flush=True)
for line in sys.stdin:
 f=json.loads(line);log.write(json.dumps(f)+'\n');kind=f['type'];rid=f['requestId']
 if kind=='warm':
  if os.environ.get('BOOT_HANG'):continue
  emit('queued',rid);emit('warmed',rid,capabilities={})
 elif kind=='open':
  scope=f['sessionId']
  if f.get('loadSessionId')=='unsupported':emit('error',rid,code='resume_unsupported')
  else:emit('queued',rid);emit('opened',rid,adapterSessionId='provider-'+scope,session={'models':['fixture-model']},capabilities={})
 elif kind=='configure':
  emit('queued',rid);emit('configured',rid,modeReady=f.get('model')!='unsupported',configuration={'model':{'status':'applied','value':f.get('model')}})
 elif kind=='prompt':
  active=rid;emit('queued',rid);emit('started',rid,adapterSessionId='provider-'+scope);text=f['text']
  if text=='permission':emit('permission',rid,permissionId='permission-exact',request={'options':[{'optionId':'provider:allow/once','kind':'allow_once'},{'optionId':'provider:deny','kind':'reject_once'}]})
  elif text=='hang':pass
  elif text=='exit':os._exit(9)
  elif text=='large-reply':emit('activity',rid,update={'text':'x'*16384});record(rid)
  elif text=='wrong-session':emit('activity',rid,sessionId='other',update={})
  elif text=='wrong-request':emit('activity','unknown',update={})
  elif text=='wrong-order':emit('activity',rid,sequence=1,update={})
  elif text=='overflow':
   for i in range(80):emit('activity',rid,update={'text':str(i)})
   record(rid)
  else:emit('activity',rid,update={'text':'reply'});record(rid)
 elif kind=='permission':
  emit('ack',rid);emit('permission_resolved',active,permissionId=f['permissionId'],optionId=f.get('optionId'));record(active)
 elif kind=='cancel':emit('ack',rid);record(f['target'],'cancelled')
 elif kind=='status':emit('status',rid,running=active)
"#;
