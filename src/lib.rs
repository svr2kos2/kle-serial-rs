#![warn(missing_docs, dead_code)]
#![warn(clippy::all, clippy::pedantic, clippy::cargo)]

//! A Rust library for deserialising [Keyboard Layout Editor] files. Designed to be used in
//! conjunction with [`serde_json`] to deserialize JSON files exported from KLE.
//!
//! # Example
//!
//! ![example]
//!
//! ```
//! // kle_serial::Keyboard uses f64 coordinates by default. If you need f32 coordinates use
//! // kle_serial::Keyboard<f32> or kle_serial::f32::Keyboard instead.
//! use kle_serial::Keyboard;
//!
//! let keyboard: Keyboard = serde_json::from_str(
//!     r#"[
//!         {"name": "example"},
//!         [{"f": 4}, "!\n1\n¹\n¡"]
//!     ]"#
//! ).unwrap();
//!
//! assert_eq!(keyboard.metadata.name, "example");
//! assert_eq!(keyboard.keys.len(), 1);
//!
//! assert!(keyboard.keys[0].legends[0].is_some());
//! let legend = keyboard.keys[0].legends[0].as_ref().unwrap();
//!
//! assert_eq!(legend.text, "!");
//! assert_eq!(legend.size, 4);
//!
//! assert!(keyboard.keys[0].legends[1].is_none());
//! ```
//!
//! [Keyboard Layout Editor]: http://www.keyboard-layout-editor.com/
//! [`serde_json`]: https://crates.io/crates/serde_json
//! [example]: https://raw.githubusercontent.com/staticintlucas/kle-serial-rs/main/doc/example.png

mod de;
pub mod f32;
pub mod f64;
mod utils;

use num_traits::real::Real;
use serde::Deserialize;

use de::{KleKeyboard, KleLayoutIterator};
use utils::FontSize;

/// Colour type used for deserialising. Type alias of [`rgb::RGBA8`].
pub type Color = rgb::RGBA8;

const NUM_LEGENDS: usize = 12; // Number of legends on a key

pub(crate) mod color {
    use crate::Color;

    pub(crate) const BACKGROUND: Color = Color::new(0xEE, 0xEE, 0xEE, 0xFF); // #EEEEEE
    pub(crate) const KEY: Color = Color::new(0xCC, 0xCC, 0xCC, 0xFF); // #CCCCCC
    pub(crate) const LEGEND: Color = Color::new(0x00, 0x00, 0x00, 0xFF); // #000000
}

/// A struct representing a single legend.
///
/// <div class="warning">
///
/// This is also referred to as a `label` in the official TypeScript [`kle-serial`] library as well
/// as some others. It is named `Legend` here to follow the more prevalent terminology and to match
/// KLE's own UI.
///
/// [`kle-serial`]: https://github.com/ijprest/kle-serial
///
/// </div>
#[derive(Debug, Clone, PartialEq)]
pub struct Legend {
    /// The legend's text.
    pub text: String,
    /// The legend size (in KLE's font size unit). KLE clamps this to the range `1..=9`.
    pub size: usize,
    /// The legend colour.
    pub color: Color,
}

impl Default for Legend {
    fn default() -> Self {
        Self {
            text: String::default(),
            size: usize::from(FontSize::default()),
            color: color::LEGEND,
        }
    }
}

/// A struct representing a key switch.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Switch {
    /// The switch mount. Typically either `"cherry"` or `"alps"`.
    pub mount: String,
    /// The switch brand. KLE uses lowercase brand names.
    pub brand: String,
    /// The switch type. KLE uses either part number or colour depending on the brand.
    pub typ: String,
}

/// A struct representing a single key.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Key<T = f64>
where
    T: Real,
{
    /// The key's legends. This array is indexed in left to right, top to bottom order as shown in
    /// the image below.
    ///
    /// ![alignment]
    ///
    /// Legends that are empty in KLE will be deserialised as [`None`].
    ///
    /// [alignment]: https://raw.githubusercontent.com/staticintlucas/kle-serial-rs/main/doc/alignment.png
    pub legends: Vec<Option<Legend>>,
    /// The colour of the key
    pub color: Color,
    /// The X position of the key measured in keyboard units (typically 19.05 mm or 0.75 in).
    ///
    /// <div class="warning">
    ///
    /// KLE has some strange behaviour when it comes to positioning stepped and L-shaped keys.
    /// The 'true' X position of the top left corner will be less if the key's `x2` field is
    /// negative.
    ///
    /// The actual position of the top left corner can be found using:
    ///
    /// ```
    /// # let key = kle_serial::Key::<f64>::default();
    /// let x = key.x.min(key.x + key.x2);
    /// ```
    ///
    /// This behaviour can be observed by placing an ISO enter in the top left corner in KLE;
    /// `x` is 0.25 and `x2` is &minus;0.25.
    ///
    /// </div>
    pub x: T,
    /// The Y position of the key measured in keyboard units (typically 19.05 mm or 0.75 in).
    ///
    /// <div class="warning">
    ///
    /// KLE has some strange behaviour when it comes to positioning stepped and L-shaped keys.
    /// The 'true' Y position of the top left corner will be less if the key's `y2` field is
    /// negative.
    ///
    /// The actual position of the top left corner can be found using:
    ///
    /// ```
    /// # let key = kle_serial::Key::<f64>::default();
    /// let y = key.y.min(key.y + key.y2);
    /// ```
    ///
    /// This behaviour can be observed by placing an ISO enter in the top left corner in KLE;
    /// `x` is 0.25 and `x2` is &minus;0.25.
    ///
    /// </div>
    pub y: T,
    /// The width of the key measured in keyboard units (typically 19.05 mm or 0.75 in).
    pub width: T,
    /// The height of the key measured in keyboard units (typically 19.05 mm or 0.75 in).
    pub height: T,
    /// The relative X position of a stepped or L-shaped part of the key. Measured in keyboard units
    /// (typically 19.05 mm or 0.75 in).
    ///
    /// This is set to `0.0` for regular keys, but is used for stepped caps lock and ISO enter keys,
    /// amongst others.
    pub x2: T,
    /// The relative Y position of a stepped or L-shaped part of the key. Measured in keyboard units
    /// (typically 19.05 mm or 0.75 in).
    ///
    /// This is set to `0.0` for regular keys, but is used for stepped caps lock and ISO enter keys,
    /// amongst others.
    pub y2: T,
    /// The width of a stepped or L-shaped part of the key. Measured in keyboard units (typically
    /// 19.05 mm or 0.75 in).
    ///
    /// This is equal to `width` for regular keys, but is used for stepped caps lock and ISO
    /// enter keys, amongst others.
    pub width2: T,
    /// The height of a stepped or L-shaped part of the key. Measured in keyboard units (typically
    /// 19.05 mm or 0.75 in).
    ///
    /// This is equal to `height` for regular keys, but is used for stepped caps lock and ISO
    /// enter keys, amongst others.
    pub height2: T,
    /// The rotation of the key in degrees. Positive rotation values are clockwise.
    pub rotation: T,
    /// The X coordinate for the centre of rotation of the key. Measured in keyboard units
    /// (typically 19.05 mm or 0.75 in) from the top left corner of the layout.
    pub rx: T,
    /// The Y coordinate for the centre of rotation of the key. Measured in keyboard units
    /// (typically 19.05 mm or 0.75 in) from the top left corner of the layout.
    pub ry: T,
    /// The keycap profile and row number of the key.
    ///
    /// KLE uses special rendering for `"SA"`, `"DSA"`, `"DCS"`, `"OEM"`, `"CHICKLET"`, and `"FLAT"`
    /// profiles. It expects the row number to be one of `"R1"`, `"R2"`, `"R3"`, `"R4"`, `"R5"`, or
    /// `"SPACE"`, although it only uses special rendering for `"SPACE"`.
    ///
    /// KLE suggests the format `"<profile> [<row>]"`, but it will recognise any string containing
    /// one of its supported profiles and/or rows. Any value is considered valid, but empty or
    /// unrecognised values are rendered using the unnamed default profile.
    pub profile: String,
    /// The key switch.
    pub key_switch: Switch,
    /// Whether the key is ghosted.
    pub ghosted: bool,
    /// Whether the key is stepped.
    pub stepped: bool,
    /// Whether this is a homing key.
    pub homing: bool,
    /// Whether this is a decal.
    pub decal: bool,
}

impl<T> Default for Key<T>
where
    T: Real,
{
    fn default() -> Self {
        Self {
            legends: vec![None; NUM_LEGENDS],
            color: color::KEY,
            x: T::zero(),
            y: T::zero(),
            width: T::one(),
            height: T::one(),
            x2: T::zero(),
            y2: T::zero(),
            width2: T::one(),
            height2: T::one(),
            rotation: T::zero(),
            rx: T::zero(),
            ry: T::zero(),
            profile: String::new(),
            key_switch: Switch::default(),
            ghosted: false,
            stepped: false,
            homing: false,
            decal: false,
        }
    }
}

/// The background style of a KLE layout.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Background {
    /// The name of the background.
    ///
    /// When generated by KLE, this is the same as the name shown in the dropdown menu, for example
    /// `"Carbon fibre 1"`.
    pub name: String,
    /// The CSS style of the background.
    ///
    /// When generated by KLE, this sets the CSS [`background-image`] property to a relative url
    /// where the associated image is located. For example the *Carbon fibre 1* background will set
    /// `style` to `"background-image: url('/bg/carbonfibre/carbon_texture1879.png');"`.
    ///
    /// [`background-image`]: https://developer.mozilla.org/en-US/docs/Web/CSS/background-image
    pub style: String,
}

/// The metadata for the keyboard layout.
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    /// Background colour for the layout.
    pub background_color: Color,
    /// Background style information for the layout.
    pub background: Background,
    /// Corner radii for the background using CSS [`border-radius`] syntax.
    ///
    /// [`border-radius`]: https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius
    pub radii: String,
    /// The name of the layout.
    pub name: String,
    /// The author of the layout.
    pub author: String,
    /// The default switch type used in this layout. This can be set separately for individual keys.
    pub key_switch: Switch,
    /// Whether the switch is plate mounted.
    pub plate_mount: bool,
    /// Whether the switch is PCB mounted.
    pub pcb_mount: bool,
    /// Notes for the layout. KLE expects GitHub-flavoured Markdown and can render this using the
    /// *preview* button, but any string data is considered valid.
    pub notes: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            background_color: color::BACKGROUND,
            background: Background::default(),
            radii: String::new(),
            name: String::new(),
            author: String::new(),
            key_switch: Switch::default(),
            plate_mount: false,
            pcb_mount: false,
            notes: String::new(),
        }
    }
}

/// A keyboard deserialised from a KLE JSON file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Keyboard<T = f64>
where
    T: Real,
{
    /// Keyboard layout's metadata.
    pub metadata: Metadata,
    /// The layout's keys.
    pub keys: Vec<Key<T>>,
}

impl<'de, T> Deserialize<'de> for Keyboard<T>
where
    T: Real + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let KleKeyboard { meta, layout } = KleKeyboard::deserialize(deserializer)?;

        Ok(Self {
            metadata: meta.into(),
            keys: KleLayoutIterator::new(layout).collect(),
        })
    }
}

/// An iterator of [`Key`]s deserialised from a KLE JSON file.
#[derive(Debug, Clone)]
pub struct KeyIterator<T = f64>(KleLayoutIterator<T>)
where
    T: Real;

impl<'de, T> Deserialize<'de> for KeyIterator<T>
where
    T: Real + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let KleKeyboard { meta: _, layout } = KleKeyboard::deserialize(deserializer)?;

        Ok(Self(KleLayoutIterator::new(layout)))
    }
}

impl<T> Iterator for KeyIterator<T>
where
    T: Real,
{
    type Item = Key<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use isclose::assert_is_close;

    use super::*;

    #[test]
    fn test_legend_default() {
        let legend = Legend::default();

        assert_eq!(legend.text, "");
        assert_eq!(legend.size, 3);
        assert_eq!(legend.color, Color::new(0, 0, 0, 255));
    }

    #[test]
    fn test_key_default() {
        let key = <Key>::default();

        for leg in key.legends {
            assert!(leg.is_none());
        }
        assert_eq!(key.color, Color::new(204, 204, 204, 255));
        assert_is_close!(key.x, 0.0);
        assert_is_close!(key.y, 0.0);
        assert_is_close!(key.width, 1.0);
        assert_is_close!(key.height, 1.0);
        assert_is_close!(key.x2, 0.0);
        assert_is_close!(key.y2, 0.0);
        assert_is_close!(key.width2, 1.0);
        assert_is_close!(key.height2, 1.0);
        assert_is_close!(key.rotation, 0.0);
        assert_is_close!(key.rx, 0.0);
        assert_is_close!(key.ry, 0.0);
        assert_eq!(key.profile, "");
        assert_eq!(key.key_switch.mount, "");
        assert_eq!(key.key_switch.brand, "");
        assert_eq!(key.key_switch.typ, "");
        assert!(!key.ghosted);
        assert!(!key.stepped);
        assert!(!key.homing);
        assert!(!key.decal);
    }

    #[test]
    fn test_metadata_default() {
        let meta = Metadata::default();

        assert_eq!(meta.background_color, Color::new(238, 238, 238, 255));
        assert_eq!(meta.background.name, "");
        assert_eq!(meta.background.style, "");
        assert_eq!(meta.radii, "");
        assert_eq!(meta.name, "");
        assert_eq!(meta.author, "");
        assert_eq!(meta.key_switch.mount, "");
        assert_eq!(meta.key_switch.brand, "");
        assert_eq!(meta.key_switch.typ, "");
        assert!(!meta.plate_mount);
        assert!(!meta.pcb_mount);
        assert_eq!(meta.notes, "");
    }

    #[test]
    fn test_keyboard_deserialize() {
        let kb: Keyboard = serde_json::from_str(
            r#"[
                {
                    "name": "test",
                    "unknown": "key"
                },
                [
                    {
                        "a": 4,
                        "unknown2": "key"
                    },
                    "A",
                    "B",
                    "C"
                ],
                [
                    "D"
                ]
            ]"#,
        )
        .unwrap();
        assert_eq!(kb.metadata.name, "test");
        assert_eq!(kb.keys.len(), 4);

        let kb: Keyboard = serde_json::from_str(r#"[["A"]]"#).unwrap();
        assert_eq!(kb.metadata.name, "");
        assert_eq!(kb.keys.len(), 1);

        let kb: Keyboard = serde_json::from_str(r#"[{"notes": "'tis a test"}]"#).unwrap();
        assert_eq!(kb.metadata.notes, "'tis a test");
        assert_eq!(kb.keys.len(), 0);

        assert!(serde_json::from_str::<Keyboard>("null").is_err());
    }

    #[test]
    fn test_key_iterator_deserialize() {
        let keys: Vec<_> = serde_json::from_str::<KeyIterator>(
            r#"[
                {
                    "name": "test",
                    "unknown": "key"
                },
                [
                    {
                        "a": 4,
                        "unknown2": "key"
                    },
                    "A",
                    "B",
                    "C"
                ],
                [
                    "D"
                ]
            ]"#,
        )
        .unwrap()
        .collect();

        assert_eq!(keys.len(), 4);
        assert_eq!(keys[2].legends[0].as_ref().unwrap().text, "C");

        let keys: Vec<_> = serde_json::from_str::<KeyIterator>(r#"[["A"]]"#)
            .unwrap()
            .collect();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].legends[0].as_ref().unwrap().text, "A");

        let keys: Vec<_> = serde_json::from_str::<KeyIterator>(r#"[{"notes": "'tis a test"}]"#)
            .unwrap()
            .collect();
        assert_eq!(keys.len(), 0);

        assert!(serde_json::from_str::<KeyIterator>("null").is_err());
    }
}
