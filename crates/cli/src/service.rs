use crate::prompt::{PromptContext, PromptRendering};
use mbx_protocol::{PromptFlags, Request, RequestKind, ResponseKind};

/// Application port consumed by transport adapters.
///
/// Implementations return response content only. The transport that decoded the
/// request remains responsible for correlation IDs and wire framing.
pub trait RequestHandler {
    fn handle(&self, request: Request) -> ResponseKind;
}

/// Maps wire requests to application behavior without knowing their transport.
pub struct ProtocolService<'a> {
    renderer: &'a dyn PromptRendering,
}

impl<'a> ProtocolService<'a> {
    pub fn new(renderer: &'a dyn PromptRendering) -> Self {
        Self { renderer }
    }
}

impl RequestHandler for ProtocolService<'_> {
    fn handle(&self, request: Request) -> ResponseKind {
        match request.kind {
            RequestKind::Ping => ResponseKind::Pong,
            RequestKind::Prompt(prompt) => {
                ResponseKind::Prompt(self.renderer.render_prompt(&PromptContext {
                    cwd: prompt.cwd,
                    status: prompt.status,
                    duration_ms: prompt.duration_ms,
                    flags: PromptFlags::from_bits(prompt.flags),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mbx_protocol::{FLAG_SSH, PromptRequest};
    use std::cell::RefCell;

    #[derive(Default)]
    struct RecordingRenderer {
        contexts: RefCell<Vec<PromptContext>>,
    }

    impl PromptRendering for RecordingRenderer {
        fn render_prompt(&self, context: &PromptContext) -> String {
            self.contexts.borrow_mut().push(context.clone());
            "rendered prompt".to_owned()
        }
    }

    struct PanicRenderer;

    impl PromptRendering for PanicRenderer {
        fn render_prompt(&self, _context: &PromptContext) -> String {
            panic!("PING must not invoke prompt rendering")
        }
    }

    #[test]
    fn ping_does_not_invoke_prompt_rendering() {
        let service = ProtocolService::new(&PanicRenderer);

        let kind = service.handle(Request {
            id: 41,
            kind: RequestKind::Ping,
        });

        assert_eq!(kind, ResponseKind::Pong);
    }

    #[test]
    fn prompt_request_maps_every_context_field_to_the_renderer() {
        const UNKNOWN_FLAG: u32 = 1 << 31;

        let renderer = RecordingRenderer::default();
        let service = ProtocolService::new(&renderer);
        let expected_context = PromptContext {
            cwd: "/work".to_owned(),
            status: 127,
            duration_ms: Some(2_500),
            flags: PromptFlags::from_bits(FLAG_SSH | UNKNOWN_FLAG),
        };

        let kind = service.handle(Request {
            id: 42,
            kind: RequestKind::Prompt(PromptRequest {
                cwd: expected_context.cwd.clone(),
                status: expected_context.status,
                duration_ms: expected_context.duration_ms,
                flags: expected_context.flags.bits(),
            }),
        });

        assert_eq!(kind, ResponseKind::Prompt("rendered prompt".to_owned()));
        assert_eq!(renderer.contexts.borrow().as_slice(), &[expected_context]);
    }
}
