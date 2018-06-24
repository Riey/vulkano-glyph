use std::sync::Arc;

use rusttype::gpu_cache::{Cache, CacheBuilder, TextureCoords};
use rusttype::PositionedGlyph;
use vulkano::buffer::CpuBufferPool;
use vulkano::command_buffer::{
    AutoCommandBuffer, AutoCommandBufferBuilder, CommandBuffer, CommandBufferExecFuture,
};
use vulkano::device::{Device, DeviceOwned, Queue};
use vulkano::format::R8Unorm;
use vulkano::image::{AttachmentImage, ImageUsage};
use vulkano::sync::NowFuture;

use {FontId, Result};

const INITIAL_WIDTH: u32 = 256;
const INITIAL_HEIGHT: u32 = 256;

/// Wraps `rusttype`'s cache for use with `vulkano`.
pub struct GpuCache<'font> {
    cache: Cache<'font>,
    tex: Arc<AttachmentImage<R8Unorm>>,
    buf: CpuBufferPool<u8>,
}

impl<'font> GpuCache<'font> {
    pub fn new<'a>(device: &Arc<Device>) -> Result<Self> {
        let width = INITIAL_WIDTH;
        let height = INITIAL_HEIGHT;
        let tex = AttachmentImage::with_usage(
            Arc::clone(device),
            [width, height],
            R8Unorm,
            ImageUsage {
                transfer_destination: true,
                ..ImageUsage::none()
            },
        )?;
        let buf = CpuBufferPool::upload(Arc::clone(device));
        let cache = CacheBuilder {
            width,
            height,
            ..Default::default()
        }.build();

        Ok(GpuCache { cache, tex, buf })
    }

    pub fn queue_glyph(&mut self, font_id: FontId, glyph: PositionedGlyph<'font>) {
        self.cache.queue_glyph(font_id, glyph)
    }

    pub fn cache_queued(
        &mut self,
        queue: Arc<Queue>,
    ) -> Result<CommandBufferExecFuture<NowFuture, AutoCommandBuffer>> {
        let GpuCache { cache, buf, tex } = self;
        let cmd = AutoCommandBufferBuilder::new(buf.device().clone(), queue.family())?;
        let mut cmd = Some(cmd);
        let mut err = None;
        cache.cache_queued(|rect, data| {
            if err.is_none() {
                let chunk = match buf.chunk(data.iter().cloned()) {
                    Ok(chunk) => chunk,
                    Err(e) => {
                        err = Some(e.into());
                        return;
                    }
                };

                let c = cmd.take().unwrap();
                cmd = match c.copy_buffer_to_image_dimensions(
                    chunk,
                    Arc::clone(tex),
                    [rect.min.x, rect.min.y, 0],
                    [rect.width(), rect.height(), 0],
                    0,
                    1,
                    0,
                ) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        err = Some(e.into());
                        return;
                    }
                };
            }
        })?;

        if let Some(err) = err {
            Err(err)
        } else {
            Ok(cmd.unwrap().build().unwrap().execute(queue).unwrap())
        }
    }

    pub fn rect_for(
        &self,
        font_id: FontId,
        glyph: &PositionedGlyph,
    ) -> Result<Option<TextureCoords>> {
        Ok(self.cache.rect_for(font_id, glyph)?)
    }

    pub fn image(&self) -> &Arc<AttachmentImage<R8Unorm>> {
        &self.tex
    }
}