use crate::{
    AcpHost, Error, HostContext, HostPolicy, Result, SessionBinding,
    registry::{Registry, Scope},
};
use lapis_ai_runtime::{self as runtime, FrameInput, ProcessorFuture, SessionProcessor};
use std::sync::{Arc, Mutex};

pub(crate) fn create(
    host: &AcpHost,
    context: &HostContext,
    scope: &Scope,
    binding: &SessionBinding,
) -> Result<Option<Box<dyn SessionProcessor>>> {
    if !admitted(
        &*host.policy,
        &host.state,
        context,
        scope,
        &binding.session_id,
    ) {
        return Err(Error("admission_revoked"));
    }
    let processor = host.policy.processor(context, binding)?;
    if !admitted(
        &*host.policy,
        &host.state,
        context,
        scope,
        &binding.session_id,
    ) {
        return Err(Error("admission_revoked"));
    }
    Ok(processor.map(|inner| {
        Box::new(ScopedProcessor {
            policy: host.policy.clone(),
            state: host.state.clone(),
            context: context.clone(),
            scope: scope.clone(),
            id: binding.session_id.clone(),
            inner,
        }) as Box<dyn SessionProcessor>
    }))
}
fn admitted(
    policy: &dyn HostPolicy,
    state: &Mutex<Registry>,
    context: &HostContext,
    scope: &Scope,
    id: &str,
) -> bool {
    context.window == scope.window
        && policy
            .authorize(context)
            .is_ok_and(|admission| admission == scope.admission)
        && state
            .lock()
            .is_ok_and(|state| state.slot(scope, id).is_ok())
}
struct ScopedProcessor {
    // No Arc<AcpHost>: it would prevent last-host Drop from retiring this worker.
    policy: Arc<dyn HostPolicy>,
    state: Arc<Mutex<Registry>>,
    context: HostContext,
    scope: Scope,
    id: String,
    inner: Box<dyn SessionProcessor>,
}
impl ScopedProcessor {
    fn check(&self, id: &str) -> runtime::Result<()> {
        if id == self.id && admitted(&*self.policy, &self.state, &self.context, &self.scope, id) {
            Ok(())
        } else {
            Err(runtime::Error("admission_revoked"))
        }
    }
}
impl SessionProcessor for ScopedProcessor {
    fn process<'a>(&'a mut self, input: FrameInput<'a>) -> ProcessorFuture<'a> {
        Box::pin(async move {
            self.check(input.session_id)?;
            let output = self.inner.process(input).await?;
            self.check(&self.id)?;
            Ok(output)
        })
    }
}
