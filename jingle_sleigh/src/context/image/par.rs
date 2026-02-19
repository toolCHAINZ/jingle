use crate::context::image::{ImageSection, ImageSections};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

/// Extension trait that adds parallel iteration over image sections.
///
/// Provided automatically for any [`ImageSections`] implementor whose
/// `SectionIter` yields `Send` items and the implementor itself is `Sync`.
/// In practice this means any image whose section data is backed by a
/// shared byte slice or an owned buffer.
///
/// # Example
/// ```ignore
/// use jingle_sleigh::context::image::par::ImageSectionsParExt;
/// let sections: Vec<_> = my_image.par_image_sections().collect();
/// ```
pub trait ImageSectionsParExt: ImageSections {
    /// Returns a parallel iterator over the sections of this image.
    ///
    /// The default implementation collects sequential sections into a `Vec`
    /// and parallelizes over that. Implementors with natively parallel
    /// backing stores may override this for better performance.
    fn par_image_sections(&self) -> impl ParallelIterator<Item = ImageSection<'_>>
    where
        Self: Sync,
    {
        self.image_sections()
            .collect::<Vec<_>>()
            .into_par_iter()
    }
}

/// Blanket implementation: any `ImageSections` type gets parallel iteration.
impl<T: ImageSections> ImageSectionsParExt for T {}
