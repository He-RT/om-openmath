//! Standalone plot requests use the same clock/flag/budget as notebook sampling.
use super::Session;
use crate::{plot::PlotError, protocol::*};
use om_core::Interrupt;
impl Session {
    pub(super) fn sample_plot(&mut self, request: PlotRequest) -> Response {
        self.reset_interrupt();
        let start = self.clock.as_ref().map(|clock| clock.now_ms());
        let ctx = Interrupt {
            flag: self.interrupt.clone(),
            clock: self.clock.clone(),
            deadline_ms: start
                .map(|ms| ms + self.config.general.eval_timeout_ms as f64)
                .filter(|ms| ms.is_finite()),
            ..Interrupt::default()
        };
        match crate::plot::sample(&request, &self.eval, &ctx) {
            Ok(data) => Response::Plot { data },
            Err(e) => Response::Error {
                message: self.plot_error(&e).1,
            },
        }
    }
    pub(super) fn plot_error(&self, error: &PlotError) -> (&'static str, String) {
        if let Some(abort) = error.abort() {
            return self.execution_error(&om_eval::EvalError::Abort(abort));
        }
        (
            "err.plot",
            self.localized(
                &format!("err.plot: 无法绘图：{error}"),
                &format!("err.plot: Cannot plot: {error}"),
            ),
        )
    }
}
