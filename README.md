# screenshot-rs
Get a bitmap image of any display in Rust. This crate is hosted on [crates.io](https://crates.io/crates/screenshot).

Contributions welcome!

## Examples

```rust
extern crate image;
extern crate screenshot;
use screenshot::get_screenshot;

fn main() {
	let s = get_screenshot(0).unwrap();

	println!("{} x {}", s.width(), s.height());

	image::save_buffer(&Path::new("test.png"),
		s.as_slice(), s.width() as u32, s.height() as u32, image::RGBA(8))
	.unwrap();
}
```

## Development
* screenshot-rs has its own systems bindings. It should migrate to [servo/rust-core-graphics](https://github.com/servo/rust-core-graphics) and [retep998/winapi-rs](https://github.com/retep998/winapi-rs). I want to use [klutzy/rust-windows](https://github.com/klutzy/rust-windows), but it doesn't have the right bindings.

## Known Issues
* Can leak memory on certain failure conditions.
