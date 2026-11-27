//! Sample the input method on the Windows UI thread before checking or editing.

use std::{ffi::c_void, ptr::null_mut};

use windows::{
    Win32::UI::{
        Input::{
            Ime::{GCS_COMPSTR, ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext},
            KeyboardAndMouse::GetFocus,
        },
        TextServices::{ITfContextComposition, ITfThreadMgr},
    },
    core::{HRESULT, Interface},
};

windows::core::link!("msctf.dll" "system" fn TF_GetThreadMgr(manager: *mut *mut c_void) -> HRESULT);

pub(super) fn composing() -> bool {
    // SAFETY: called on the native UI thread. COM pointers stay on that thread,
    // and the legacy input context is always released after the query.
    unsafe {
        let mut manager = null_mut();
        if TF_GetThreadMgr(&mut manager).is_ok() && !manager.is_null() {
            let manager = ITfThreadMgr::from_raw(manager);
            if let Ok(context) = manager.GetFocus().and_then(|doc| doc.GetTop())
                && let Ok(compositions) = context.cast::<ITfContextComposition>()
                && let Ok(items) = compositions.EnumCompositions()
            {
                let mut item = [None];
                let mut fetched = 0;
                if items.Next(&mut item, &mut fetched).is_ok() && fetched != 0 {
                    return true;
                }
            }
        }
        let window = GetFocus();
        if window.is_invalid() {
            return false;
        }
        let input = ImmGetContext(window);
        if input.is_invalid() {
            return false;
        }
        let bytes = ImmGetCompositionStringW(input, GCS_COMPSTR, None, 0);
        let _ = ImmReleaseContext(window, input);
        bytes > 0
    }
}
