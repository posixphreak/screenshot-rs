#![allow(unstable)]
#![feature(core)]

extern crate screenshot;
extern crate bmp;
extern crate image;

use screenshot::get_screenshot;
use bmp::{Image, Pixel};
use std::path::Path;

fn main() {
	// X = Row (Width), Y = Col (Height)
	let s = get_screenshot(0).unwrap();

	println!("{} x {} x {} = {} bytes", s.width(), s.height(), s.pixel_width(), s.raw_len());

	let origin = s.get_pixel(0, 0);
	println!("(0,0): R: {}, G: {}, B: {}", origin.r, origin.g, origin.b);

	let end_col = s.get_pixel(0, s.height()-1);
	println!("(0,end): R: {}, G: {}, B: {}", end_col.r, end_col.g, end_col.b);

	let opp = s.get_pixel(s.width()-1, s.height()-1);
	println!("(end,end): R: {}, G: {}, B: {}", opp.r, opp.g, opp.b);


	let mut img = Image::new(s.width() as u32, s.height() as u32);
	for y in 0..s.height() {
		for x in 0..s.width() {
			let p = s.get_pixel(x, y);
			img.set_pixel(x as u32, y as u32, Pixel {r: p.r, g: p.g, b: p.b});
		}
	}
	img.save("test.bmp");

	image::save_buffer(&Path::new("test.png"),
		s.as_slice(), s.width() as u32, s.height() as u32, image::RGBA(8))
	.unwrap();
}
