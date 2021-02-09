use crate::{device::GraphicDevice, errors, texture::Texture};
use glow::HasContext;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct TexturePack {
    /// Texture atlases that have space available for
    /// more textures.
    open: Vec<(Texture, Packer)>,
    /// Full atlases.
    closed: Vec<Texture>,
    /// Minimum size of newly allocated textures.
    min_size: [u32; 2],
    padding: u32,
}

impl TexturePack {
    /// Default dimension, width or height, of each texture in texels.
    ///
    /// - OpenGL 4 requires support of at least 1024.
    /// - OpenGL ES 3 requires support of at least 2048;
    pub const DEFAULT_DIM: u32 = 1024;

    pub fn new(device: &GraphicDevice) -> errors::Result<Self> {
        // This is the maximum addressable texture dimension.
        // Doesn't mean the device has enough memory to allocate
        // such a texture, though.
        let max_size = unsafe { device.gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) };
        println!("GL_MAX_TEXTURE_SIZE: {}", max_size);

        Self::with_size(device, Self::DEFAULT_DIM, Self::DEFAULT_DIM)
    }

    pub fn with_size(device: &GraphicDevice, width: u32, height: u32) -> errors::Result<Self> {
        Ok(Self {
            open: vec![(
                Texture::new(device, width, height)?,
                Packer::new(width, width),
            )],
            closed: vec![],
            min_size: [width, height],
            padding: 1,
        })
    }

    pub fn add_image_data(
        &mut self,
        device: &GraphicDevice,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> errors::Result<Texture> {
        // Upfront validations.
        if width == 0 || height == 0 {
            return Err(crate::errors::Error::InvalidTextureSize(width, height));
        }

        let expected_len = width as usize * height as usize * 4;
        println!("expected {}, actual {}", expected_len, data.len());
        if expected_len != data.len() {
            return Err(crate::errors::Error::InvalidImageData {
                expected: expected_len,
                actual: data.len(),
            });
        }

        let [padded_width, padded_height] = [width + self.padding * 2, height + self.padding * 2];

        // Look for a texture with space.
        for (texture, packer) in &mut self.open {
            if let Some(slot_pos) = packer.try_insert(padded_width, padded_height) {
                let [padded_x, padded_y] = [slot_pos[0] + self.padding, slot_pos[1] + self.padding];
                texture.update_sub_data(device, [padded_x, padded_y], [width, height], data)?;
                return Ok(texture.new_sub([padded_x, padded_y], [width, height])?);
            }
        }

        // No available space left in open set.
        // TODO: validate device requirements that dimensions be a factor of 2
        let new_tex_width = padded_width.min(Self::DEFAULT_DIM);
        let new_tex_height = padded_height.min(Self::DEFAULT_DIM);
        self.open.push((
            Texture::new(device, new_tex_width, new_tex_height)?,
            Packer::new(new_tex_width, new_tex_height),
        ));
        let maybe_new = self.open.last_mut().and_then(|(texture, packer)| {
            packer
                .try_insert(padded_width, padded_height)
                .map(|slot| (texture, slot))
        });

        // A new texture was allocated with enough space. If
        // the packer did not find a slot, it's a bug.
        debug_assert!(maybe_new.is_some());

        let (texture, slot_pos) = maybe_new.unwrap();
        let [padded_x, padded_y] = [slot_pos[0] + self.padding, slot_pos[1] + self.padding];
        texture.update_sub_data(device, [padded_x, padded_y], [width, height], data)?;

        Ok(texture.new_sub([padded_x, padded_y], [width, height])?)
    }
}

/// Rectangle based bin packer.
///
/// # Examples
///
/// # Implementation
///
/// ```text
///  ____________________________
/// |          |                 |
/// |   Slot   |      Right      |
/// |          |                 |
/// |__________|_________________|
/// |                            |
/// |                            |
/// |           Bottom           |
/// |                            |
/// |                            |
/// |____________________________|
/// ```
struct Packer {
    rects: Vec<RectNode>,
    available: u32,
    padding: u32,
}

impl Packer {
    fn new(width: u32, height: u32) -> Self {
        // Packer starts with a root node that covers the
        // entire available space.
        let root = RectNode::Leaf(Rectangle {
            pos: [0, 0],
            size: [width, height],
        });

        Self {
            rects: vec![root],
            available: 1,
            padding: 0,
        }
    }

    fn has_space(&self) -> bool {
        self.available > 0
    }

    fn try_insert(&mut self, width: u32, height: u32) -> Option<[u32; 2]> {
        if self.rects.is_empty() {
            return None;
        }

        self.insert_internal([width, height], 0)
    }

    /// Internal recursive insert.
    fn insert_internal(&mut self, target: [u32; 2], index: usize) -> Option<[u32; 2]> {
        // Clone needed to avoid double borrow when splitting
        // a leaf into a branch. Not optimal, but the enum is
        // relatively small and shouldn't incur too much of
        // a performance penalty.
        match self.rects[index].clone() {
            RectNode::Vacant => unreachable!("Recursion followed leaf to non-existing node."),
            RectNode::Closed => {
                // Node's rectangle is considered too small to contain anything.
                None
            }
            RectNode::Leaf(rect) => {
                if rect.can_fit(target) {
                    // Success. Claim this node as an available slot
                    // for the target, and split the remaining area
                    // into a rectangle to the right, and a rectangle
                    // to the bottom.
                    // TODO: Padding
                    let slot = rect.pos;

                    // Claim node for the target.
                    self.rects[index] = RectNode::Branch(Rectangle {
                        pos: rect.pos,
                        size: target,
                    });

                    // Split into an implicit branch.
                    let right = index * 2 + 1;
                    let bottom = index * 2 + 2;

                    // Ensure that vector can contain the
                    // children at the expected indices.
                    if bottom >= self.rects.len() {
                        self.rects.resize_with(bottom + 1, || RectNode::Vacant);
                    }

                    self.set_child_rect(
                        right,
                        Rectangle {
                            pos: [slot[0] + target[1], slot[1]],
                            size: [rect.size[0] - target[0], target[1]],
                        },
                    );
                    self.set_child_rect(
                        bottom,
                        Rectangle {
                            pos: [slot[0], slot[1] + target[1]],
                            size: [rect.size[0], rect.size[1] - target[1]],
                        },
                    );

                    self.available -= 1;
                    Some(slot)
                } else {
                    // Vacant node is too small for what
                    // we're trying to insert.
                    None
                }
            }
            RectNode::Branch(_) => {
                // Recursive search into right and bottom branches.
                // Right node takes precedent.
                self.insert_internal(target, index * 2 + 1)
                    // Try bottom node if right fails.
                    .or_else(|| self.insert_internal(target, index * 2 + 2))
            }
        }
    }

    fn set_child_rect(&mut self, index: usize, rect: Rectangle) {
        // TODO: Configurable minimum
        if rect.size[0] > 0 && rect.size[1] > 0 {
            self.rects[index] = RectNode::Leaf(rect);
            self.available += 1;
        } else {
            self.rects[index] = RectNode::Closed;
        }
    }
}

#[derive(Debug, Clone)]
enum RectNode {
    /// Space in the binary heap for the child nodes
    /// of a potential branch, which hasn't been split
    /// yet.
    ///
    /// Consider this scenario. The root node, index 0,
    /// is occupied and split into right node 1 and bottom
    /// node 2.
    ///
    /// An insert is attempted into node 1, but fails to
    /// find a fit. A fit is found in node 2, which is
    /// split into nodes 5 and 6.
    ///
    /// Node 1's children would be node 3 and 4, however
    /// it is still vacant, that is it's still a leaf and
    /// not a branch. The vector must contain some value
    /// and node 2 must have its children at the expected
    /// indices.
    ///
    /// This is where `Vacant` comes in, indicating space
    /// for children nodes that don't exist yet.
    ///
    /// ```text
    ///           +-----------v---v
    ///   +---v---v
    /// | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
    /// | B | L | B | V | V | L | L |
    ///       +-------^---^
    /// ```
    Vacant,

    /// Leaf node that has no space. This can happen
    /// when the slot is too small to hold an image.
    Closed,

    /// Leaf node of the tree structure, which does not
    /// contain an image. It can accept an image and be
    /// split further.
    Leaf(Rectangle),

    /// Branch node that contains a rectangle slot, and
    /// implicitly refers to two child nodes.
    Branch(Rectangle),
}

#[derive(Debug, Clone)]
#[deprecated]
struct Rectangle {
    pos: [u32; 2],
    size: [u32; 2],
}

impl Rectangle {
    fn can_fit(&self, other: [u32; 2]) -> bool {
        other[0] <= self.size[0] && other[1] <= self.size[1]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pack() {
        let mut packer = Packer::new(100, 100);

        assert_eq!(packer.try_insert(50, 50), Some([0, 0]));
        assert_eq!(packer.available, 2);
        assert!(packer.has_space());

        assert_eq!(packer.try_insert(50, 50), Some([50, 0]));
        assert_eq!(packer.available, 1);
        assert!(packer.has_space());

        assert_eq!(packer.try_insert(50, 50), Some([0, 50]));
        assert_eq!(packer.available, 1);
        assert!(packer.has_space());

        assert_eq!(packer.try_insert(50, 50), Some([50, 50]));
        assert_eq!(packer.available, 0);
        assert!(!packer.has_space());
    }
}
