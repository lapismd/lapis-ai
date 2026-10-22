#![allow(dead_code)]
use lapis_ai_service::{
    AcpHost, Config, Error, HostContext, HostPolicy, Launch, Result, Target,
    runtime::{Environment, PreparedLaunch, SessionRequest, SessionSnapshot},
};
use serde_json::Value;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
pub struct Policy {
    pub root: PathBuf,
    pub generation: AtomicU64,
    pub blocked: AtomicBool,
    pub bad_cleanup: AtomicBool,
    pub slow_shutdown: AtomicBool,
}
impl HostPolicy for Policy {
    fn authorize(&self, context: &HostContext) -> Result<String> {
        if !["one", "two"].contains(&context.window.as_str()) || context.url != "tauri://localhost/"
        {
            return Err(Error("denied"));
        }
        Ok(format!("vault-{}", self.generation.load(Ordering::Acquire)))
    }
    fn targets(&self, _: &HostContext) -> Result<Vec<Target>> {
        Ok(vec![Target {
            id: "fixture".into(),
            label: "Fixture".into(),
        }])
    }
    fn resolve(
        &self,
        context: &HostContext,
        target: &str,
        resume_binding: Option<&str>,
    ) -> Result<Launch> {
        if target != "fixture" {
            return Err(Error("denied"));
        }
        let resume = match resume_binding {
            None => None,
            Some("owned-binding") if self.generation.load(Ordering::Acquire) == 0 => {
                Some("owned-provider-session".into())
            }
            _ => return Err(Error("resume_not_admitted")),
        };
        let root = self.root.join(&context.window);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("peer.py");
        std::fs::write(&path, PEER).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut environment = Environment::default();
        environment
            .0
            .insert("PATH".into(), std::env::var("PATH").unwrap());
        environment.0.insert(
            "LOG".into(),
            root.join("log.jsonl").to_string_lossy().into(),
        );
        if self.blocked.load(Ordering::Acquire) {
            environment.0.insert("BLOCK".into(), "1".into());
        }
        if self.bad_cleanup.load(Ordering::Acquire) {
            environment.0.insert("FAIL_CLEANUP".into(), "1".into());
        }
        if self.slow_shutdown.load(Ordering::Acquire) {
            environment.0.insert("SLOW_SHUTDOWN".into(), "1".into());
        }
        Ok(Launch {
            prepared: PreparedLaunch {
                executable: path,
                arguments: vec![],
                directory: root.clone(),
                environment,
            },
            directory: root,
            resume,
        })
    }
}
pub fn setup(root: &Path) -> (Arc<AcpHost>, Arc<Policy>) {
    let policy = Arc::new(Policy {
        root: root.into(),
        generation: AtomicU64::new(0),
        blocked: AtomicBool::new(false),
        bad_cleanup: AtomicBool::new(false),
        slow_shutdown: AtomicBool::new(false),
    });
    (
        Arc::new(AcpHost::new(policy.clone(), Config::default()).unwrap()),
        policy,
    )
}
pub fn context(window: &str) -> HostContext {
    HostContext {
        window: window.into(),
        url: "tauri://localhost/".into(),
    }
}
pub fn log(root: &Path, window: &str) -> Vec<Value> {
    std::fs::read_to_string(root.join(window).join("log.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
pub fn alive(pid: &str) -> bool {
    std::process::Command::new("/bin/kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
        .success()
}
pub fn all_stopped(root: &Path, window: &str) -> bool {
    log(root, window)
        .iter()
        .filter_map(|frame| frame["pid"].as_u64())
        .all(|pid| !alive(&pid.to_string()))
}
pub async fn wait_start(root: &Path, window: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !root.join(window).join("log.jsonl").exists()
            || !log(root, window)
                .iter()
                .any(|frame| frame["type"] == "warm")
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
pub async fn wait(
    host: &AcpHost,
    ctx: &HostContext,
    id: &str,
    predicate: impl Fn(&SessionSnapshot) -> bool,
) -> SessionSnapshot {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = host.snapshot(ctx, id, 0).unwrap();
            if predicate(&snapshot) {
                return snapshot;
            }
            assert!(!snapshot.closed, "closed: {:?}", snapshot.failure);
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}
pub async fn configure(host: &AcpHost, ctx: &HostContext, id: &str) {
    host.submit(
        ctx,
        id,
        "configure".into(),
        SessionRequest::Configure {
            model: None,
            effort: None,
            mode: Some("ask".into()),
        },
    )
    .await
    .unwrap();
    wait(host, ctx, id, |s| {
        s.events
            .iter()
            .any(|event| event.frame["type"] == "configured")
    })
    .await;
}
pub fn prompt() -> SessionRequest {
    SessionRequest::Prompt {
        text: "hello".into(),
        context: None,
        max_duration_ms: 1000,
    }
}
const PEER: &str = r#"#!/usr/bin/env python3
import sys,os,json,time
log=open(os.environ['LOG'],'a',buffering=1)
log.write(json.dumps({'pid':os.getpid()})+'\n')
scope=None
provider=None
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
 if kind=='warm':
  if os.environ.get('BLOCK'):sys.stdin.read();sys.exit(7 if os.environ.get('FAIL_CLEANUP') else 0)
  emit('warmed',rid)
 elif kind=='open':scope=f['sessionId'];provider=f.get('loadSessionId') or 'provider-'+scope;emit('opened',rid,adapterSessionId=provider,session={})
 elif kind=='configure':emit('configured',rid,modeReady=True)
 elif kind=='prompt':
  emit('started',rid,adapterSessionId=provider)
  emit('activity',rid,update={'content':{'text':'reply'}})
  emit('record',rid,record={'version':1,'taskId':rid,'status':'completed'},sessionUsable=True,quiescent=True)
 elif kind=='status':emit('status',rid)
if os.environ.get('SLOW_SHUTDOWN'):time.sleep(0.5)
"#;
