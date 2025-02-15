// Copyright (C) 2021 Scott Lamb <slamb@slamb.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fixed-size audio sample codecs as defined in
//! [RFC 3551 section 4.5](https://datatracker.ietf.org/doc/html/rfc3551#section-4.5).

use std::num::NonZeroU32;

use bytes::Bytes;

use super::CodecItem;

#[derive(Debug)]
pub(crate) struct Depacketizer {
    clock_rate: u32,
    pending: Option<super::AudioFrame>,
    bits_per_sample: u32,
}

impl Depacketizer {
    /// Creates a new Depacketizer.
    pub(super) fn new(clock_rate: u32, bits_per_sample: u32) -> Self {
        Self {
            clock_rate,
            bits_per_sample,
            pending: None,
        }
    }

    pub(super) fn parameters(&self) -> Option<super::Parameters> {
        Some(super::Parameters::Audio(super::AudioParameters {
            rfc6381_codec: None,
            frame_length: None, // variable
            clock_rate: self.clock_rate,
            extra_data: Bytes::new(),
            sample_entry: None,
        }))
    }

    fn frame_length(&self, payload_len: usize) -> Option<NonZeroU32> {
        // This calculation could be strength-reduced but it's just once per frame anyway.
        // Let's do it in a straightforward way.
        assert!(payload_len < usize::from(u16::MAX));
        let bits = (payload_len) as u32 * 8;
        match (bits % self.bits_per_sample) != 0 {
            true => None,
            false => NonZeroU32::new(bits / self.bits_per_sample),
        }
    }

    pub(super) fn push(&mut self, pkt: crate::client::rtp::Packet) -> Result<(), String> {
        assert!(self.pending.is_none());
        let frame_length = self.frame_length(pkt.payload.len()).ok_or_else(|| {
            format!(
                "invalid length {} for payload of {}-bit audio samples",
                pkt.payload.len(),
                self.bits_per_sample
            )
        })?;
        self.pending = Some(super::AudioFrame {
            loss: pkt.loss,
            ctx: pkt.ctx,
            stream_id: pkt.stream_id,
            timestamp: pkt.timestamp,
            frame_length,
            data: pkt.payload,
        });
        Ok(())
    }

    pub(super) fn pull(&mut self) -> Option<super::CodecItem> {
        self.pending.take().map(CodecItem::AudioFrame)
    }
}
