use crate::colors::ColorPalette;
use rpa_core::{Activity, ActivityMetadata};
use std::borrow::Cow;

pub trait ActivityExt {
    fn get_name(&self) -> Cow<'static, str>;
    fn get_color(&self) -> egui::Color32;
}

impl ActivityExt for Activity {
    fn get_name(&self) -> Cow<'static, str> {
        rust_i18n::t!(ActivityMetadata::for_activity(self).name_key)
    }

    fn get_color(&self) -> egui::Color32 {
        ColorPalette::for_activity(self)
    }
}
