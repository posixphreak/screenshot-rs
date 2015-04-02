#![allow(unused_must_use)]

extern crate screenshot;
extern crate bmp;
extern crate image;

use std::path::Path;

fn main() {
	// X = Row (Width), Y = Col (Height)
	let s = screenshot::get_screenshot(0).unwrap();

	println!("{} x {} x {} = {} bytes", s.width(), s.height(), s.pixel_width(), s.raw_len());

	let origin = s.get_pixel(0, 0);
	println!("(0,0): R: {}, G: {}, B: {}", origin.r, origin.g, origin.b);

	let end_col = s.get_pixel(0, s.height()-1);
	println!("(0,end): R: {}, G: {}, B: {}", end_col.r, end_col.g, end_col.b);

	let opp = s.get_pixel(s.width()-1, s.height()-1);
	println!("(end,end): R: {}, G: {}, B: {}", opp.r, opp.g, opp.b);

	let mut img = bmp::Image::new(s.width() as u32, s.height() as u32);
	// Rebuild data because Piston/image swaps R & B channels
	let mut data = Vec::<u8>::new();
	for y in 0..s.height() {
		for x in 0..s.width() {
			let p = s.get_pixel(x, y);
			img.set_pixel(x as u32, y as u32, bmp::Pixel {r: p.r, g: p.g, b: p.b});

			data.push(p.r);
			data.push(p.g);
			data.push(p.b);
			data.push(p.a);
		}
	}
	img.save("test.bmp");

	image::save_buffer(&Path::new("test.png"), &data,
		s.width() as u32, s.height() as u32, image::RGBA(8)).unwrap();
}
