//! Embedded application assets.
//!
//! In-app icons are baked into the binary at compile time so an installed
//! NumNum is fully relocatable. Loading them from the filesystem would tie the
//! binary to the build machine's source tree, which breaks every package.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

const SETTINGS_SVG: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/settings.svg"));
const BURGER_SVG: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/burger.svg"));

/// Asset source for icons rendered inside the app. Resolves the relative paths
/// passed to `svg().path(...)`; registered via `Application::with_assets`.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            "icons/settings.svg" => Ok(Some(Cow::Borrowed(SETTINGS_SVG))),
            "icons/burger.svg" => Ok(Some(Cow::Borrowed(BURGER_SVG))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(vec![
            SharedString::from("icons/settings.svg"),
            SharedString::from("icons/burger.svg"),
        ])
    }
}
