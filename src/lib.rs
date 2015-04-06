
//! Capture a bitmap image of a display. The resulting screenshot is stored in
//! the `Screenshot` type, which varies per platform.
//!
//! # Platform-specific details
//!
//! Despite OS X's CoreGraphics documentation, the bitmap returned has its
//! origin at the top left corner. It uses ARGB pixels.
//!
//! The Windows GDI bitmap has its coordinate origin at the bottom left. We
//! attempt to undo this by reordering the rows. Windows also uses ARGB pixels.
#![feature(core, libc, collections)]

extern crate libc;

use std::intrinsics::size_of;
pub use ffi::{get_screenshot};

#[derive(Clone, Copy)]
pub struct Pixel {
	pub a: u8,
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

/// An image buffer containing the screenshot.
pub struct Screenshot {
	data: Vec<u8>,
	height: u32,
	width: u32,
	row_len: u32, // Might be superfluous
	pixel_width: u32,
}

impl Screenshot {
	/// Height of image in pixels.
	#[inline]
	pub fn height(&self) -> u32 { self.height }

	/// Width of image in pixels.
	#[inline]
	pub fn width(&self) -> u32 { self.width }

	/// Number of bytes in one row of bitmap.
	#[inline]
	pub fn row_len(&self) -> u32 { self.row_len }

	/// Width of pixel in bytes.
	#[inline]
	pub fn pixel_width(&self) -> u32 { self.pixel_width }

	/// Raw bitmap.
	#[inline]
	pub unsafe fn raw_data(&self) -> *const u8 {
		&self.data[0] as *const u8
	}

	/// Number of bytes in bitmap
	#[inline]
	pub fn raw_len(&self) -> u32 {
		self.data.len() * unsafe {size_of::<u8>()} as u32
	}

	/// Gets pixel at (row, col)
	pub fn get_pixel(&self, row: u32, col: u32) -> Pixel {
		let idx = (col * self.row_len() + row * self.pixel_width()) as usize;
		// let the std library do the bounds checking
		Pixel {
			a: self.data[idx+3],
			r: self.data[idx+2],
			g: self.data[idx+1],
			b: self.data[idx],
		}
	}
}

pub type ScreenResult = Result<Screenshot, &'static str>;

#[cfg(target_os = "linux")]
mod ffi {
	extern crate xlib;

	use ::{Screenshot, ScreenResult};
	use std::ptr::null_mut;
	use std::mem;
	use libc::{c_int, c_uint};
	use self::xlib::{XOpenDisplay, XCloseDisplay, XScreenOfDisplay, XRootWindowOfScreen,
		XDestroyWindow, XWindowAttributes, XGetWindowAttributes, XImage, XGetImage, XAllPlanes, ZPixmap};

	pub fn get_screenshot(screen: u32) -> ScreenResult {
		unsafe {
			let display = XOpenDisplay(null_mut());
			let screen = XScreenOfDisplay(display, screen as c_int);
			let root = XRootWindowOfScreen(screen);

			let mut attr: XWindowAttributes = mem::uninitialized();
			XGetWindowAttributes(display, root, &mut attr);

			let mut img = &mut *XGetImage(display, root, 0, 0, attr.width as c_uint, attr.height as c_uint,
				XAllPlanes(), ZPixmap);
			XDestroyWindow(display, root);
			XCloseDisplay(display);
			// This is the function which XDestroyImage macro calls.
			// servo/rust-xlib doesn't handle function pointers correctly.
			// We have to transmute the variable.
			let destroy_image: extern fn(*mut XImage) -> c_int = mem::transmute(img.f.destroy_image);
			let height = img.height as u32;
			let width = img.width as u32;
			let row_len = img.bytes_per_line as u32;
			let pixel_bits = img.bits_per_pixel as u32;
			if pixel_bits % 8 != 0 {
				destroy_image(&mut *img);
				return Err("Pixels aren't integral bytes.");
			}
			let pixel_width = pixel_bits / 8;

			// Create a Vec for image
			let size = width * height * pixel_width;
			let mut data = Vec::<u8>::from_raw_buf(img.data as *mut u8, size as usize);
			destroy_image(&mut *img);

			// Fix Alpha channel when xlib cannot retrieve info correctly
			let has_alpha = data.iter().enumerate().any(|(n, x)| n % 4 == 3 && *x != 0);
			if !has_alpha {
				let mut n = 0;
				for channel in &mut data {
					if n % 4 == 3 { *channel = 255; }
					n += 1;
				}
			}

			Ok(Screenshot {
				data: data,
				height: height,
				width: width,
				row_len: row_len,
				pixel_width: pixel_width,
			})
		}
	}
}

#[cfg(target_os = "macos")]
mod ffi {
	#![allow(non_upper_case_globals, dead_code)]

	use libc;
	use ::Screenshot;
	use ::ScreenResult;

	type CFIndex = libc::c_long;
	type CFDataRef = *const u8; // *const CFData

	#[cfg(target_arch = "x86")]
	type CGFloat = libc::c_float;
	#[cfg(target_arch = "x86_64")]
	type CGFloat = libc::c_double;
	type CGError = libc::int32_t;

	type CGDirectDisplayID = libc::uint32_t;
	type CGDisplayCount = libc::uint32_t;
	type CGImageRef = *mut u8; // *mut CGImage
	type CGDataProviderRef = *mut u8; // *mut CGDataProvider

	const kCGErrorSuccess: CGError = 0;
	const kCGErrorFailure: CGError = 1000;
	const CGDisplayNoErr: CGError = kCGErrorSuccess;

	#[link(name = "CoreGraphics", kind = "framework")]
	extern "C" {
		fn CGGetActiveDisplayList(max_displays: libc::uint32_t,
								  active_displays: *mut CGDirectDisplayID,
								  display_count: *mut CGDisplayCount) -> CGError;
		fn CGDisplayCreateImage(displayID: CGDirectDisplayID) -> CGImageRef;
		fn CGImageRelease(image: CGImageRef);

		fn CGImageGetBitsPerComponent(image: CGImageRef) -> libc::size_t;
		fn CGImageGetBitsPerPixel(image: CGImageRef) -> libc::size_t;
		fn CGImageGetBytesPerRow(image: CGImageRef) -> libc::size_t;
		fn CGImageGetDataProvider(image: CGImageRef) -> CGDataProviderRef;
		fn CGImageGetHeight(image: CGImageRef) -> libc::size_t;
		fn CGImageGetWidth (image: CGImageRef) -> libc::size_t;

		fn CGDataProviderCopyData(provider: CGDataProviderRef) -> CFDataRef;
	}
	#[link(name = "CoreFoundation", kind = "framework")]
	extern "C" {
		fn CFDataGetLength (theData: CFDataRef) -> CFIndex;
		fn CFDataGetBytePtr(theData: CFDataRef) -> *const u8;
		fn CFRelease(cf: *const libc::c_void);
	}

	/// Get a screenshot of the requested display.
	pub fn get_screenshot(screen: usize) -> ScreenResult {
		unsafe {
			// Get number of displays
			let mut count: CGDisplayCount = 0;
			let mut err = CGDisplayNoErr;
			err = CGGetActiveDisplayList(0, 0 as *mut CGDirectDisplayID, &mut count);
			if err != CGDisplayNoErr {
				return Err("Error getting number of displays.");
			}

			// Get list of displays
			let mut disps: Vec<CGDisplayCount> = Vec::with_capacity(count as usize);
			disps.set_len(count as usize);
			err = CGGetActiveDisplayList(disps.len() as libc::uint32_t,
				&mut disps[0] as *mut CGDirectDisplayID,
				&mut count);
			if err != CGDisplayNoErr {
				return Err("Error getting list of displays.");
			}

			// Get screenshot of requested display
			let disp_id = disps[screen];
			let cg_img = CGDisplayCreateImage(disp_id);

			// Get info about image
			let width = CGImageGetWidth(cg_img) as usize;
			let height = CGImageGetHeight(cg_img) as usize;
			let row_len = CGImageGetBytesPerRow(cg_img) as usize;
			let pixel_bits = CGImageGetBitsPerPixel(cg_img) as usize;
			if pixel_bits % 8 != 0 {
				return Err("Pixels aren't integral bytes.");
			}

			// Copy image into a Vec buffer
			let cf_data = CGDataProviderCopyData(CGImageGetDataProvider(cg_img));
			let raw_len = CFDataGetLength(cf_data) as usize;
			if width*height*pixel_bits != raw_len*8 {
				return Err("Image size is inconsistent with W*H*D.");
			}
			let data = Vec::<u8>::from_raw_buf(CFDataGetBytePtr(cf_data), raw_len);

			// Release native objects
			CGImageRelease(cg_img);
			CFRelease(cf_data as *const libc::c_void);

			Ok(Screenshot {
				data: data,
				height: height,
				width: width,
				row_len: row_len,
				pixel_width: pixel_bits/8
			})
		}
	}
}

#[cfg(target_os = "windows")]
mod ffi {
	#![allow(non_snake_case, dead_code)]

	use libc::{c_int, c_uint, c_long, c_void};
	use std::intrinsics::{size_of};

	use ::Screenshot;
	use ::ScreenResult;

	type PVOID = *mut c_void;
	type LPVOID = *mut c_void;
	type WORD = u16; // c_uint;
	type DWORD = u32; // c_ulong;
	type BOOL = c_int;
	type BYTE = u8;
	type UINT = c_uint;
	type LONG = c_long;
	type LPARAM = c_long;

	#[repr(C)]
	struct RECT {
		left: LONG,
		top: LONG,
		right: LONG, // immediately outside rect
		bottom: LONG, // immediately outside rect
	}
	type LPCRECT = *const RECT;
	type LPRECT = *mut RECT;

	type HANDLE = PVOID;
	type HMONITOR = HANDLE;
	type HWND = HANDLE;
	type HDC = HANDLE;
	#[repr(C)]
	struct MONITORINFO {
		cbSize: DWORD,
		rcMonitor: RECT,
		rcWork: RECT,
		dwFlags: DWORD,
	}
	type LPMONITORINFO = *mut MONITORINFO;
	type MONITORENUMPROC = fn(HMONITOR, HDC, LPRECT, LPARAM) -> BOOL;

	type HBITMAP = HANDLE;
	type HGDIOBJ = HANDLE;
	type LPBITMAPINFO = PVOID; // Hack

	const NULL: *mut c_void = 0us as *mut c_void;
	const HGDI_ERROR: *mut c_void = -1is as *mut c_void;
	const SM_CXSCREEN: c_int = 0;
	const SM_CYSCREEN: c_int = 1;

	/// TODO verify value
	const SRCCOPY: u32 = 0x00CC0020;
	const CAPTUREBLT: u32 = 0x40000000;
	const DIB_RGB_COLORS: UINT = 0;
	const BI_RGB: DWORD = 0;

	#[repr(C)]
	struct BITMAPINFOHEADER {
		biSize: DWORD,
		biWidth: LONG,
		biHeight: LONG,
		biPlanes: WORD,
		biBitCount: WORD,
		biCompression: DWORD,
		biSizeImage: DWORD,
		biXPelsPerMeter: LONG,
		biYPelsPerMeter: LONG,
		biClrUsed: DWORD,
		biClrImportant: DWORD,
	}

	#[repr(C)]
	struct RGBQUAD {
		rgbBlue: BYTE,
		rgbGreen: BYTE,
		rgbRed: BYTE,
		rgbReserved: BYTE,
	}

	/// WARNING variable sized struct
	#[repr(C)]
	struct BITMAPINFO {
		bmiHeader: BITMAPINFOHEADER,
		bmiColors: [RGBQUAD; 1],
	}

	#[link(name = "user32")]
	extern "system" {
		fn GetSystemMetrics(m: c_int) -> c_int;
		fn EnumDisplayMonitors(hdc: HDC, lprcClip: LPCRECT,
							   lpfnEnum: MONITORENUMPROC, dwData: LPARAM) -> BOOL;
		fn GetMonitorInfo(hMonitor: HMONITOR, lpmi: LPMONITORINFO) -> BOOL;
		fn GetDesktopWindow() -> HWND;
		fn GetDC(hWnd: HWND) -> HDC;
	}

	#[link(name = "gdi32")]
	extern "system" {
		fn CreateCompatibleDC(hdc: HDC) -> HDC;
		fn CreateCompatibleBitmap(hdc: HDC, nWidth: c_int, nHeight: c_int) -> HBITMAP;
		fn SelectObject(hdc: HDC, hgdiobj: HGDIOBJ) -> HGDIOBJ;
		fn BitBlt(hdcDest: HDC, nXDest: c_int, nYDest: c_int, nWidth: c_int, nHeight: c_int,
				  hdcSrc: HDC, nXSrc: c_int, nYSrc: c_int, dwRop: DWORD) -> BOOL;
		fn GetDIBits(hdc: HDC, hbmp: HBITMAP, uStartScan: UINT, cScanLines: UINT,
					 lpvBits: LPVOID, lpbi: LPBITMAPINFO, uUsage: UINT) -> c_int;

		fn DeleteObject(hObject: HGDIOBJ) -> BOOL;
		fn ReleaseDC(hWnd: HWND, hDC: HDC) -> c_int;
		fn DeleteDC(hdc: HDC) -> BOOL;
	}

	/// Reorder rows in bitmap, last to first.
	/// TODO rewrite functionally
	fn flip_rows(data: Vec<u8>, height: usize, row_len: usize) -> Vec<u8> {
		let mut new_data = Vec::with_capacity(data.len());
		unsafe {new_data.set_len(data.len())};
		for row_i in range(0, height) {
			for byte_i in range(0, row_len) {
				let old_idx = (height-row_i-1)*row_len + byte_i;
				let new_idx = row_i*row_len + byte_i;
				new_data[new_idx] = data[old_idx];
			}
		}
		new_data
	}

	/// TODO Support multiple screens
	/// This may never happen, given the horrific quality of Win32 APIs
	pub fn get_screenshot(_screen: usize) -> ScreenResult {
		unsafe {
			// Enumerate monitors, getting a handle and DC for requested monitor.
			// loljk, because doing that on Windows is worse than death
			let h_wnd_screen = GetDesktopWindow();
			let h_dc_screen = GetDC(h_wnd_screen);
			let width = GetSystemMetrics(SM_CXSCREEN);
			let height = GetSystemMetrics(SM_CYSCREEN);

			// Create a Windows Bitmap, and copy the bits into it
			let h_dc = CreateCompatibleDC(h_dc_screen);
			if h_dc == NULL { return Err("Can't get a Windows display.");}

			let h_bmp = CreateCompatibleBitmap(h_dc_screen, width, height);
			if h_bmp == NULL { return Err("Can't create a Windows buffer");}

			let res = SelectObject(h_dc, h_bmp);
			if res == NULL || res == HGDI_ERROR {
				return Err("Can't select Windows buffer.");
			}

			let res = BitBlt(h_dc, 0, 0, width, height, h_dc_screen, 0, 0, SRCCOPY|CAPTUREBLT);
			if res == 0 { return Err("Failed to copy screen to Windows buffer");}

			// Get image info
			let pixel_width: usize = 4; // FIXME
			let mut bmi = BITMAPINFO {
				bmiHeader: BITMAPINFOHEADER {
					biSize: size_of::<BITMAPINFOHEADER>() as DWORD,
					biWidth: width as LONG,
					biHeight: height as LONG,
					biPlanes: 1,
					biBitCount: 8*pixel_width as WORD,
					biCompression: BI_RGB,
					biSizeImage: (width * height * pixel_width as c_int) as DWORD,
					biXPelsPerMeter: 0,
					biYPelsPerMeter: 0,
					biClrUsed: 0,
					biClrImportant: 0,
				},
				bmiColors: [RGBQUAD {
					rgbBlue: 0,
					rgbGreen: 0,
					rgbRed: 0,
					rgbReserved: 0
				}],
			};

			// Create a Vec for image
			let size: usize = (width*height) as usize * pixel_width;
			let mut data: Vec<u8> = Vec::with_capacity(size);
			data.set_len(size);

			// copy bits into Vec
			GetDIBits(h_dc, h_bmp, 0, height as DWORD,
				&mut data[0] as *mut u8 as *mut c_void,
				&mut bmi as *mut BITMAPINFO as *mut c_void,
				DIB_RGB_COLORS);

			// Release native image buffers
			ReleaseDC(h_wnd_screen, h_dc_screen); // don't need screen anymore
			DeleteDC(h_dc);
			DeleteObject(h_bmp);

			let data = flip_rows(data, height as usize, width as usize*pixel_width);

			Ok(Screenshot {
				data: data,
				height: height as usize,
				width: width as usize,
				row_len: width as usize*pixel_width,
				pixel_width: pixel_width,
			})
		}
	}
}

#[test]
fn test_get_screenshot() {
	let s: Screenshot = get_screenshot(0).unwrap();
	println!("width: {}\n height: {}\npixel width: {}\n bytes: {}",
		s.width(), s.height(), s.pixel_width(), s.raw_len());
}
