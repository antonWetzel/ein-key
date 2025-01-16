use gpui::*;

pub struct BundledAssets;

macro_rules! create_match {
    ($path:expr, $($file:literal),*) => {
        match $path {
        	$($file => Ok(Some(include_bytes!(concat!("../assets/", $file)).into())),)*
            _ => Ok(None),
        }
    };
}

impl AssetSource for BundledAssets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        create_match!(path, "check.svg", "chevron-right.svg", "plus.svg", "x.svg")
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        todo!("List assets for path {:?}", path);
    }
}
