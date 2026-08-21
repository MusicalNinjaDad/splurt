//! Tagging utilities for FLAC metadata
//!
//! # Tracing
//!
//! This module currently does not emit tracing spans as it only contains
//! trait implementations for metadata manipulation.

use metaflac::block::{Picture, PictureType};
use zune_jpeg::{JpegDecoder, zune_core::bytestream::ZCursor};

pub trait PictureExt {
    fn from_jpeg<B: AsRef<[u8]>, S: ToString>(
        picture_type: PictureType,
        description: S,
        data: B,
    ) -> Self;
}

impl PictureExt for Picture {
    fn from_jpeg<B: AsRef<[u8]>, S: ToString>(
        picture_type: PictureType,
        description: S,
        data: B,
    ) -> Self {
        let mut jpg_info = JpegDecoder::new(ZCursor::from(&data));
        let _ = jpg_info.decode_headers();
        let width = jpg_info.info().unwrap().width as u32;
        let height = jpg_info.info().unwrap().height as u32;
        Picture {
            picture_type,
            mime_type: "image/jpeg".to_string(),
            description: description.to_string(),
            width,
            height,
            depth: 24,
            num_colors: 0,
            data: data.as_ref().to_vec(),
        }
    }
}
