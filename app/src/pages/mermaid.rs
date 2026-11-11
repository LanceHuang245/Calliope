//! The Mermaid diagrams of the rendered note (```` ```mermaid ```` code):
//! drawn by `merman` as SVG, shown as an image; code it cannot draw shows
//! with the reason.

use std::{cell::RefCell, collections::HashMap, panic, sync::OnceLock};

use merman::{
    OperationControl, RenderOutput, RenderRequest, Renderer, SvgEnvironment, SvgRequest,
    svg::SvgPipeline,
};
use merman_export::{NativeFontContext, NativeFontLimits, NativeFontOptions};
use nsh_base::{MarkdownExtension, text_view::CustomNode, theme};
use nsh_core::{element::ImageSrc, prelude::*};

use crate::strings;

/// The name of the custom nodes Mermaid code is read as.
const MERMAID: &str = "mermaid";
/// At most this many diagrams are kept drawn.
const KEPT: usize = 64;

/// What a diagram's code draws, or why it cannot be drawn.
type Drawn = Result<String, String>;

thread_local! {
    /// The diagrams drawn, by code and appearance: the text view makes a
    /// diagram's block again when the blocks before it change, and it then
    /// shows at once, drawing nothing again.
    static DRAWN: RefCell<HashMap<(String, bool), Drawn>> = RefCell::default();
}

/// Reads fenced code of the language `mermaid` as a diagram; other code
/// keeps its highlighting.
pub(crate) fn mermaid_blocks() -> MarkdownExtension {
    MarkdownExtension::language(MERMAID, "mermaid")
}

/// A diagram of the rendered note, drawn off the UI thread, and again when
/// the appearance changes; one drawn before shows at once.
pub(crate) fn mermaid(node: &CustomNode) -> AnyElement {
    let source = node.text.clone();
    let drawn = signal(untracked(|| cached(&source, dark())));
    effect({
        let source = source.clone();
        move || {
            let (source, dark) = (source.clone(), dark());
            if let Some(hit) = cached(&source, dark) {
                return drawn.set(Some(hit));
            }
            spawn(async move {
                let key = source.clone();
                let done = background(async move { draw(&source, dark) }).await;
                let done = done.unwrap_or_else(|e| Err(e.to_string()));
                remember(key, dark, done.clone());
                drawn.set(Some(done));
            });
        }
    });
    dynamic(move || {
        drawn.get().map(|drawn| match drawn {
            Ok(markup) => svg(ImageSrc::svg_markup(&markup))
                .max_w(relative(1.0))
                .into_element(),
            Err(error) => view()
                .gap_1()
                .child(
                    text(error)
                        .text_sm()
                        .text_color(move || theme().destructive),
                )
                .child(
                    text(source.clone())
                        .text_sm()
                        .text_color(move || theme().muted_foreground),
                )
                .into_element(),
        })
    })
    .into_element()
}

fn cached(source: &str, dark: bool) -> Option<Drawn> {
    DRAWN.with(|d| d.borrow().get(&(source.to_owned(), dark)).cloned())
}

/// Keeps what `source` draws; past [`KEPT`] diagrams, the others go.
fn remember(source: String, dark: bool, drawn: Drawn) {
    DRAWN.with(|d| {
        let mut d = d.borrow_mut();
        if d.len() >= KEPT {
            d.clear();
        }
        d.insert((source, dark), drawn);
    });
}

/// The system's fonts, found once: the labels are measured and drawn with
/// them.
fn fonts() -> Result<NativeFontContext, String> {
    static FONTS: OnceLock<Result<NativeFontContext, String>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            // A desktop with many fonts has more than merman reads by default.
            let limits = NativeFontLimits {
                max_system_cmap_bytes: 256 << 20,
                ..Default::default()
            };
            NativeFontOptions::default()
                .with_system_fonts(true)
                .with_limits(limits)
                .load(&OperationControl::new())
                .map_err(|e| e.to_string())
        })
        .clone()
}

/// The diagram of Mermaid `source` as SVG markup, or why it cannot be
/// drawn. No Mermaid code makes it panic.
fn draw(source: &str, dark: bool) -> Drawn {
    panic::catch_unwind(|| {
        let fonts = fonts()?;
        let source = match dark {
            true => format!("%%{{init: {{\"theme\": \"dark\"}}}}%%\n{source}"),
            false => source.to_owned(),
        };
        let environment = SvgEnvironment::deterministic()
            .with_native_font_context(fonts.clone())
            .map_err(|e| e.to_string())?;
        let request = SvgRequest {
            pipeline: Some(SvgPipeline::resvg_safe()),
            environment,
            ..Default::default()
        };
        let request = RenderRequest::svg(&source, OperationControl::new(), request);
        let output = Renderer::new().render(request).map_err(|e| e.to_string())?;
        let RenderOutput::Svg(Some(svg)) = output else {
            return Err(strings::NO_DIAGRAM.to_owned());
        };
        // nsh draws no SVG text: the labels become paths.
        let mut options = usvg::Options::default();
        fonts.apply_to_usvg_options(&mut options);
        let tree = usvg::Tree::from_str(svg.svg(), &options).map_err(|e| e.to_string())?;
        Ok(tree.to_string(&usvg::WriteOptions::default()))
    })
    .unwrap_or_else(|_| Err(strings::DIAGRAM_FAILED.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIAGRAMS: [&str; 6] = [
        "flowchart TD\n  A[开始] --> B{判断?}\n  B -->|是| C[处理]",
        "sequenceDiagram\n  用户->>Server: 请求\n  Server-->>用户: response",
        "classDiagram\n  class Animal {\n    +String name\n    +eat() void\n  }\n  Animal <|-- Dog",
        "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Running : start\n  Running --> [*]",
        "gantt\n  dateFormat YYYY-MM-DD\n  section 设计\n  需求 :a1, 2026-11-01, 7d",
        "pie title 占比\n  \"Rust\" : 60\n  \"Swift\" : 40",
    ];

    #[test]
    fn draws_the_six_diagrams_without_text_elements() {
        for source in DIAGRAMS {
            for dark in [false, true] {
                let svg = draw(source, dark).unwrap_or_else(|e| panic!("{source}: {e}"));
                assert!(svg.contains("<path") && !svg.contains("<text"), "{source}");
            }
        }
    }

    #[test]
    fn keeps_what_was_drawn_within_bounds() {
        remember("pie".into(), false, Ok("<svg/>".into()));
        assert_eq!(cached("pie", false), Some(Ok("<svg/>".into())));
        assert_eq!(cached("pie", true), None);
        for i in 0..KEPT {
            remember(i.to_string(), false, Err(String::new()));
        }
        assert!(DRAWN.with(|d| d.borrow().len()) <= KEPT);
    }

    #[test]
    fn bad_code_is_an_error_not_a_panic() {
        for source in [
            "",
            "notADiagram\n x",
            "flowchart TD\n A --> --> [[[",
            "gantt\n x :a, 9999-99-99, -5d",
        ] {
            assert!(draw(source, false).is_err(), "{source:?}");
        }
    }
}
