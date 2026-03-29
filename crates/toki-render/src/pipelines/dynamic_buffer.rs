use wgpu::{Device, Queue};

#[derive(Debug)]
pub(crate) struct DynamicVertexBuffer {
    label: &'static str,
    buffer: Option<wgpu::Buffer>,
    capacity_bytes: usize,
}

impl DynamicVertexBuffer {
    pub(crate) fn new(label: &'static str) -> Self {
        Self {
            label,
            buffer: None,
            capacity_bytes: 0,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.buffer = None;
        self.capacity_bytes = 0;
    }

    pub(crate) fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub(crate) fn write(&mut self, device: &Device, queue: &Queue, data: &[u8]) {
        if data.is_empty() {
            self.clear();
            return;
        }

        self.ensure_capacity(device, data.len());
        if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, data);
        }
    }

    fn ensure_capacity(&mut self, device: &Device, required_capacity_bytes: usize) {
        if !Self::needs_reallocation(self.capacity_bytes, required_capacity_bytes) {
            return;
        }

        self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(self.label),
            size: required_capacity_bytes as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.capacity_bytes = required_capacity_bytes;
    }

    pub(crate) fn needs_reallocation(
        current_capacity_bytes: usize,
        required_capacity_bytes: usize,
    ) -> bool {
        current_capacity_bytes < required_capacity_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicVertexBuffer;

    #[test]
    fn needs_reallocation_when_required_capacity_exceeds_current_capacity() {
        assert!(DynamicVertexBuffer::needs_reallocation(6 * 1000, 6 * 1001));
    }

    #[test]
    fn does_not_reallocate_when_current_capacity_is_sufficient() {
        assert!(!DynamicVertexBuffer::needs_reallocation(6 * 1001, 6 * 1000));
    }
}
