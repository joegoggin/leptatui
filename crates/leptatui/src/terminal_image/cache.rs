//! Cached terminal graphics protocol data.

use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
};

use ratatui::layout::Size;
use ratatui_image::sliced::SlicedProtocol;

use super::fallback::TerminalImageFallback;

/// Cached terminal image protocol data reused across redraws.
#[derive(Default)]
pub(super) struct TerminalImageCache {
    /// Protocol data indexed by path and terminal-cell size.
    sliced_protocols: HashMap<TerminalImageCacheKey, SlicedProtocol>,
}

/// Cache key for a path-backed image rendered at a fixed terminal-cell size.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct TerminalImageCacheKey {
    /// Source image path.
    path: PathBuf,
    /// Requested terminal-cell width.
    width: u16,
    /// Requested terminal-cell height.
    height: u16,
}

impl TerminalImageCache {
    /// Returns the number of cached protocols.
    pub(super) fn len(&self) -> usize {
        self.sliced_protocols.len()
    }

    /// Returns a cached sliced protocol for the image path and requested size.
    pub(super) fn sliced_protocol(
        &mut self,
        picker: &ratatui_image::picker::Picker,
        path: &Path,
        size: Size,
    ) -> Result<&SlicedProtocol, TerminalImageFallback> {
        let key = TerminalImageCacheKey {
            path: path.to_path_buf(),
            width: size.width,
            height: size.height,
        };

        match self.sliced_protocols.entry(key) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let reader = image::ImageReader::open(path)
                    .map_err(|_| TerminalImageFallback::DecodeFailed)?;
                let image = reader
                    .decode()
                    .map_err(|_| TerminalImageFallback::DecodeFailed)?;
                let protocol = SlicedProtocol::new(picker, image, Some(size))
                    .map_err(|_| TerminalImageFallback::RenderFailed)?;
                Ok(entry.insert(protocol))
            }
        }
    }
}
