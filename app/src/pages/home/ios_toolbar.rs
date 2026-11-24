//! UIKit's interactive glass capsule and the controls it contains.

use std::{ptr::NonNull, rc::Rc};

use block2::RcBlock;
use nsh_core::JsonValue;
use nsh_platform_ios::{ComponentContext, Components, NativeComponent};
use objc2::{
    MainThreadMarker, MainThreadOnly, msg_send,
    rc::Retained,
    runtime::{AnyClass, Bool},
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::NSString;
use objc2_ui_kit::{
    UIAction, UIButton, UIButtonType, UIColor, UIControlEvents, UIImage, UIView, UIVisualEffect,
    UIVisualEffectView,
};

use super::{BAR_HEIGHT, GLASS_BUTTON, GLASS_INSET, GLASS_STRIDE, GLASS_TOOLBAR};

pub(crate) fn register(components: &Components) {
    components.register(
        GLASS_TOOLBAR,
        Rc::new(|context| {
            let mtm = MainThreadMarker::new().expect("UIKit mounts on the main thread");
            // Earlier iOS uses the same UIKit buttons without a glass effect.
            let effect = AnyClass::get(c"UIGlassEffect").map(|class| {
                // SAFETY: This is UIGlassEffect's UIKit factory and interactive property.
                let effect: Retained<UIVisualEffect> =
                    unsafe { msg_send![class, effectWithStyle: 0_isize] };
                let _: () = unsafe { msg_send![&*effect, setInteractive: Bool::YES] };
                effect
            });
            let glass = UIVisualEffectView::initWithEffect(
                UIVisualEffectView::alloc(mtm),
                effect.as_deref(),
            );
            glass.setUserInteractionEnabled(true);
            glass.layer().setCornerRadius(f64::from(BAR_HEIGHT / 2.0));
            glass.setClipsToBounds(true);
            Box::new(GlassToolbar {
                glass,
                buttons: Vec::new(),
                context,
            })
        }),
    );
}

struct GlassToolbar {
    glass: Retained<UIVisualEffectView>,
    buttons: Vec<(u64, Retained<UIButton>)>,
    context: ComponentContext,
}

impl NativeComponent for GlassToolbar {
    fn view(&self) -> &UIView {
        &self.glass
    }

    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>) {
        if name != "items" {
            return;
        }
        let items = value
            .and_then(JsonValue::as_array)
            .map_or(&[][..], Vec::as_slice);
        let items = items
            .iter()
            .filter_map(|item| Some((item["index"].as_u64()?, item)));
        // Rebuild only when visible actions change. Updating selection in place
        // lets UIKit finish the native animation on the pressed button.
        if !self
            .buttons
            .iter()
            .map(|(index, _)| *index)
            .eq(items.clone().map(|(index, _)| index))
        {
            for (_, button) in self.buttons.drain(..) {
                button.removeFromSuperview();
            }
            for (slot, (index, item)) in items.clone().enumerate() {
                let button = self.button(slot, index, item);
                self.glass.contentView().addSubview(&button);
                self.buttons.push((index, button));
            }
        }
        for ((_, button), (_, item)) in self.buttons.iter().zip(items) {
            let selected = item["selected"].as_bool().unwrap_or(false);
            button.setSelected(selected);
            let tint = if selected {
                UIColor::labelColor()
            } else {
                UIColor::secondaryLabelColor()
            };
            // SAFETY: The tint is always a UIKit color.
            unsafe { button.setTintColor(Some(&tint)) };
        }
    }
}

impl GlassToolbar {
    fn button(&self, slot: usize, index: u64, item: &JsonValue) -> Retained<UIButton> {
        let mtm = self.glass.mtm();
        let button = UIButton::buttonWithType(UIButtonType::System, mtm);
        button.setFrame(CGRect::new(
            CGPoint::new(
                f64::from(GLASS_INSET + slot as f32 * GLASS_STRIDE),
                f64::from(GLASS_INSET),
            ),
            CGSize::new(f64::from(GLASS_BUTTON), f64::from(GLASS_BUTTON)),
        ));
        let title = NSString::from_str(item["label"].as_str().unwrap_or_default());
        let symbol = NSString::from_str(item["symbol"].as_str().unwrap_or_default());
        if let Some(image) = UIImage::systemImageNamed(&symbol) {
            button.setImage_forState(Some(&image), objc2_ui_kit::UIControlState::Normal);
        } else {
            button.setTitle_forState(Some(&title), objc2_ui_kit::UIControlState::Normal);
        }
        // SAFETY: UIKit's accessibilityLabel setter takes an NSString.
        let _: () = unsafe { msg_send![&*button, setAccessibilityLabel: &*title] };
        let context = self.context.clone();
        let handler = RcBlock::new(move |_: NonNull<UIAction>| {
            context.emit("press", JsonValue::from(index));
        });
        // SAFETY: UIKit copies this block and dispatches it on the main thread.
        let action = unsafe { UIAction::actionWithHandler(RcBlock::as_ptr(&handler), mtm) };
        button.addAction_forControlEvents(&action, UIControlEvents::PrimaryActionTriggered);
        button
    }
}
