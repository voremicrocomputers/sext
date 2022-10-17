pub mod colours;

use crate::colours::TextColour;
use fontdue::layout::GlyphPosition;
use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
use fontdue::Font;
use fontdue::FontSettings;
use std::collections::HashMap;
use std::sync::Arc;
use log::debug;

/// The main text renderer struct, which holds a single font and its cache.
/// Try not to clone this as it may end up containing a large amount of data.
/// Instead, you might want to wrap this in an `Arc` or some other pointer type.
#[derive(Clone)]
pub struct TextRenderer<T> {
    pub font: Arc<Font>,
    pub layout: Arc<Layout>,
    glyph_caches: HashMap<u16, GlyphCache<T>>,
}

/// Internal struct, contains a `HashMap` of `TextColour` to a `HashMap` of `char` to (raw glyph data, `DrawableSurface`).
/// This is because, historically as SDL2 surfaces were used, it was important to keep the raw glyph data alive so that
/// less memory copying was required for SDL2 surfaces. It is thus recommended that you do not copy the raw glyph data,
/// and instead attempt to borrow it within your `DrawableSurface` implementation. (which we didn't do in our test implementation cause we were lazy)
#[derive(Clone)]
struct GlyphCache<T> {
    pub size: f32,
    pub surface_map: HashMap<TextColour, HashMap<char, (Vec<u8>, T)>>,
}

/// A "surface" that you can draw pixels to.
/// Historically, this was an SDL2 surface, but it has been abstracted out to allow for other backends.
pub trait DrawableSurface {
    /// This function will be called to "paste" a glyph upon the surface.
    /// The `x` and `y` coordinates are where the top left of the glyph should be pasted.
    /// The `width` and `height` are the dimensions of the area that the glyph should be rendered.
    /// KEEP IN MIND THAT THIS MAY NOT BE THE SAME AS THE ACTUAL GLYPH DIMENSIONS.
    /// `data` is in reference to another `DrawableSurface` that contains the glyph data.
    fn paste(&mut self, x: usize, y: usize, width: usize, height: usize, data: &Self);
    /// This function takes in raw RGBA bytes and creates a `DrawableSurface` from them.
    /// The `width` and `height` are the dimensions of the surface.
    /// The `data` parameter is a slice of bytes that contains the RGBA data.
    /// The `colour` parameter is a `TextColour` that will be used to colour the surface.
    /// There is little reason to actually care about the `colour` parameter, as it is only used for caching.
    /// Check the tests section of this library for an example of how to use this function.
    fn from_raw_mask(width: usize, height: usize, data: &[u8], colour: TextColour) -> Self;
}

/// Enum for the different (1) possible errors that you could get while constructing a TextRenderer.
#[derive(Debug, Clone, Copy)]
pub enum TextRendererError {
    FontNotFound,
}

/// Internal function to convert the fontdue grayscale bitmaps to our superior RGBA bitmaps
fn cache_glyph<T>(font: Arc<Font>, glyph: GlyphPosition, colour: TextColour, make_t: impl FnOnce(&[u8]) -> T) -> (Vec<u8>, T) {
    debug!("caching glyph: {:?}", glyph);
    let (metrics, mut bitmap) = font.rasterize_config(glyph.key);
    let mut coloured_pixels = Vec::new();
    for pixel in bitmap.iter_mut() {
        coloured_pixels.push(colour.r); // u8
        coloured_pixels.push(colour.g); // u8
        coloured_pixels.push(colour.b); // u8
        coloured_pixels.push(*pixel); // u8
    }
    // create T from bitmap
    let t = make_t(&coloured_pixels);
    (coloured_pixels, t)
}

impl<T> TextRenderer<T> where T: DrawableSurface, T: Clone {
    /// Loads a font from a specified path and creates a `TextRenderer` instance.
    /// Will return `TextRendererError::FontNotFound` if the font could not be found.
    /// Will also return a `TextRendererError::FontNotFound` if the font could not be loaded, because i haven't added other errors yet.
    pub fn load(font_path: &str) -> Result<Self, TextRendererError> {
        let font_data = std::fs::read(font_path).map_err(|_| TextRendererError::FontNotFound)?;
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|_| TextRendererError::FontNotFound)?;
        let layout = Layout::new(CoordinateSystem::PositiveYDown);
        Ok(TextRenderer {
            font: Arc::new(font),
            layout: Arc::new(layout),
            glyph_caches: HashMap::new(),
        })
    }

    /// Same as `draw_string`, but forces each character to be rendered at the same width.
    /// This can cause some minor visual artifacts, but is useful for some cases where i'm lazy.
    /// Notable warning: this will currently cause each character to have a kerning of 0.
    pub fn draw_string_monospaced(
        &mut self,
        string: &str,
        x: f32,
        y: f32,
        size: f32,
        colour: TextColour,
        surface: &mut T
    ) {
        let mut layout_settings = LayoutSettings::default();
        layout_settings.x = x;
        layout_settings.y = y;
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&layout_settings);
        layout.append(&[self.font.clone()], &TextStyle::new(string, size, 0));
        let glyphs = layout.glyphs();
        for (glyph, i) in glyphs.iter().zip(0..) {
            let bitmap = self.get_glyph_surface(*glyph, glyph.width, glyph.height, colour);
            // draw to surface
            surface.paste(
                (x + (size / 2.0) * i as f32) as usize,
                (y + glyph.y) as usize,
                (size / 2.0) as usize,
                glyph.height as usize,
                &bitmap,
            );
        }
    }

    /// Draws a string using the default settings and fontdue's layout engine.
    /// In the future, this will probably have added systems for typesetting, but for now you'll have
    /// to live without being able to set the kerning of your text.
    pub fn draw_string(
        &mut self,
        string: &str,
        x: f32,
        y: f32,
        size: f32,
        colour: TextColour,
        surface: &mut T
    ) {
        let mut layout_settings = LayoutSettings::default();
        layout_settings.x = x;
        layout_settings.y = y;
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&layout_settings);
        layout.append(&[self.font.clone()], &TextStyle::new(string, size, 0));
        let glyphs = layout.glyphs();
        for glyph in glyphs.iter() {
            let bitmap = self.get_glyph_surface(*glyph, glyph.width, glyph.height, colour);
            // draw to surface
            surface.paste(
                (x + glyph.x) as usize,
                (y + glyph.y) as usize,
                glyph.width as usize,
                glyph.height as usize,
                &bitmap,
            );
        }
    }

    /// Internal function to get the glyph drawable from either the cache or the font
    fn get_glyph_surface(
        &mut self,
        glpyh: GlyphPosition,
        width: usize,
        height: usize,
        colour: TextColour,
    ) -> T {
        let size = height as u16;
        // check if glyph cache exists
        // if not create it
        self.glyph_caches.entry(size).or_insert(GlyphCache {
            size: size as f32,
            surface_map: HashMap::new(),
        });
        // get glyph cache
        // check if colour exists
        // if not create it
        let glyph_cache = self.glyph_caches.get_mut(&size).unwrap();
        glyph_cache.surface_map.entry(colour).or_insert_with(|| HashMap::new());
        // get colour map
        // check if glyph exists
        // if not create it
        let colour_map = glyph_cache.surface_map.get_mut(&colour).unwrap();
        if let std::collections::hash_map::Entry::Vacant(e) = colour_map.entry(glpyh.parent) {
            e.insert(cache_glyph(self.font.clone(), glpyh, colour, |data| T::from_raw_mask(width, height, data, colour)));
        }
        // get glyph surface
        let glyph_surface = colour_map.get(&glpyh.parent).unwrap();
        // return glyph surface
        glyph_surface.1.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use super::*;

    #[derive(Debug, Clone)]
    struct TestSurface {
        width: usize,
        height: usize,
        data: Vec<u8>,
    }

    impl DrawableSurface for TestSurface {
        fn paste(&mut self, x: usize, y: usize, width: usize, height: usize, data: &Self) {
            println!("paste: x: {}, y: {}, width: {}, height: {}, data: {:?}", x, y, width, height, data);
            // data contains an rgba bitmap
            let data_pitch = data.width as i32 * 4;
            let pitch = self.width as i32 * 4;
            let mut data_index = 0i32;
            let mut index = (y as i32 * pitch) + (x as i32 * 4);
            // WIDTH AND DATA WIDTH ARE DIFFERENT
            for _ in 0..height {
                for _ in 0..width {
                    // if we're out of bounds on either surface, skip
                    if index < 0 || index >= (self.width * self.height * 4) as i32 || data_index < 0 || data_index >= (data.width * data.height * 4) as i32 {
                        data_index += 4;
                        index += 4;
                        continue;
                    }
                    self.data[index as usize] = data.data[data_index as usize];
                    self.data[index as usize + 1] = data.data[data_index as usize + 1];
                    self.data[index as usize + 2] = data.data[data_index as usize + 2];
                    self.data[index as usize + 3] = data.data[data_index as usize + 3];
                    data_index += 4;
                    index += 4;
                }
                index += pitch - (width as i32 * 4);
                data_index += data_pitch - (width as i32 * 4);
            }
        }
        // data is rgba
        fn from_raw_mask(width: usize, height: usize, data: &[u8], colour: TextColour) -> Self {
            println!("from_raw_mask");
            println!("width: {}", width);
            println!("height: {}", height);
            println!("data: {:?}", data);
            TestSurface {
                width,
                height,
                data: data.to_vec(),
            }
        }
    }

    #[test]
    fn test_text_renderer() {
        let mut renderer = TextRenderer::load("FreeMono.ttf").unwrap();
        let mut surface = TestSurface {
            width: 256,
            height: 256,
            data: vec![0; 256 * 256 * 4],
        };
        renderer.draw_string_monospaced("hElLo w0r1d!", 0.0, 0.0, 24.0, TextColour::new_rgb(255, 255, 255), &mut surface);
        renderer.draw_string("hElLo w0r1d!", 0.0, 24.0, 24.0, TextColour::new_rgb(255, 255, 255), &mut surface);
        // convert from rgba to rgb
        let mut rgb_data = Vec::new();
        for i in 0..(surface.width * surface.height) {
            // if transparent, put black; otherwise, put the pixel
            if surface.data[i * 4 + 3] == 0 {
                rgb_data.push(0);
                rgb_data.push(0);
                rgb_data.push(0);
            } else {
                rgb_data.push(surface.data[i * 4]);
                rgb_data.push(surface.data[i * 4 + 1]);
                rgb_data.push(surface.data[i * 4 + 2]);
            }
        }
        // output to bitmap
        let mut file = std::fs::File::create("test.ppm").unwrap();
        let _ = file.write(format!("P6\n{} {}\n255\n", surface.width, surface.height).as_bytes()).unwrap();
        let _ = file.write(&rgb_data).unwrap();
    }
}