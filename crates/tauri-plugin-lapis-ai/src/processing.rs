use crate::{
    AcpHost, Error, HostContext, HostPolicy, Result, SessionBinding,
    registry::{Registry, Scope},
};
use lapis_ai_runtime::{
    self as runtime, BatchProcessorFuture, FrameInput, ProcessorFuture, SessionProcessor,
};
use std::sync::{Arc, Mutex};

pub(crate) fn create(
    host: &AcpHost,
    context: &HostContext,
    scope: &Scope,
    binding: &SessionBinding,
) -> Result<Option<Box<dyn SessionProcessor>>> {
    if !admitted(&*host.policy, &host.state, context, scope, binding) {
        return Err(Error("admission_revoked"));
    }
    let processor = host.policy.processor(context, binding)?;
    if !admitted(&*host.policy, &host.state, context, scope, binding) {
        return Err(Error("admission_revoked"));
    }
    Ok(processor.map(|inner| {
        Box::new(ScopedProcessor {
            policy: host.policy.clone(),
            state: host.state.clone(),
            context: context.clone(),
            scope: scope.clone(),
            binding: binding.clone(),
            inner,
        }) as Box<dyn SessionProcessor>
    }))
}
fn admitted(
    policy: &dyn HostPolicy,
    state: &Mutex<Registry>,
    context: &HostContext,
    scope: &Scope,
    binding: &SessionBinding,
) -> bool {
    policy
        .authorize(context)
        .is_ok_and(|admission| admission == scope.admission)
        && owned(state, context, scope, binding)
}
fn owned(
    state: &Mutex<Registry>,
    context: &HostContext,
    scope: &Scope,
    binding: &SessionBinding,
) -> bool {
    context.window == scope.window
        && binding.admission == scope.admission
        && state.lock().is_ok_and(|state| {
            state.slot(scope, &binding.session_id).is_ok_and(|slot| {
                slot.request == binding.request_id && slot.target == binding.target_id
            })
        })
}
struct ScopedProcessor {
    // No Arc<AcpHost>: it would prevent last-host Drop from retiring this worker.
    policy: Arc<dyn HostPolicy>,
    state: Arc<Mutex<Registry>>,
    context: HostContext,
    scope: Scope,
    binding: SessionBinding,
    inner: Box<dyn SessionProcessor>,
}
impl ScopedProcessor {
    fn check(&self, id: &str) -> runtime::Result<()> {
        if id == self.binding.session_id
            && self
                .policy
                .authorize_processing(&self.context, &self.binding)
                .is_ok_and(|admission| admission == self.scope.admission)
            && owned(&self.state, &self.context, &self.scope, &self.binding)
        {
            Ok(())
        } else {
            Err(runtime::Error("admission_revoked"))
        }
    }
}
impl SessionProcessor for ScopedProcessor {
    fn process_batch<'a>(&'a mut self, input: FrameInput<'a>) -> BatchProcessorFuture<'a> {
        Box::pin(async move {
            self.check(input.session_id)?;
            let output = self.inner.process_batch(input).await?;
            self.check(&self.binding.session_id)?;
            Ok(output)
        })
    }
    fn process<'a>(&'a mut self, input: FrameInput<'a>) -> ProcessorFuture<'a> {
        Box::pin(async move {
            self.check(input.session_id)?;
            let output = self.inner.process(input).await?;
            self.check(&self.binding.session_id)?;
            Ok(output)
        })
    }
}
