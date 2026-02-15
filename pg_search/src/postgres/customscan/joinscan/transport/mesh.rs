use super::shmem::{MultiplexedDsmReader, MultiplexedDsmWriter, RingBufferHeader, SignalBridge};
use parking_lot::Mutex;
use std::sync::Arc;

/// A registry for process-local DSM communication channels.
///
/// This struct encapsulates the raw shared memory regions and the multiplexers
/// that allow this process to communicate with all other participants in the MPP session.
pub struct TransportMesh {
    /// Writers for sending data TO other participants.
    /// Index `j` corresponds to the channel sending to participant `j`.
    pub mux_writers: Vec<Arc<Mutex<MultiplexedDsmWriter>>>,

    /// Readers for receiving data FROM other participants.
    /// Index `j` corresponds to the channel receiving from participant `j`.
    pub mux_readers: Vec<Arc<Mutex<MultiplexedDsmReader>>>,

    /// The local signal bridge for this participant.
    pub bridge: Arc<SignalBridge>,
}

impl TransportMesh {
    /// Initializes the mesh from raw shared memory pointers.
    ///
    /// This function sets up the `MultiplexedDsmWriter`s and `MultiplexedDsmReader`s
    /// by slicing the monolithic shared memory region according to the standard layout:
    /// `[Participant 0 -> 0][Participant 0 -> 1]...[Participant P-1 -> P-1]`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// 1. `base_ptr` points to a valid, initialized shared memory region.
    /// 2. The region size is at least `region_size * total_participants * total_participants`.
    /// 3. The memory is accessible for the lifetime of the returned `TransportMesh`.
    pub unsafe fn init(
        base_ptr: *mut u8,
        region_size: usize,
        participant_index: usize,
        total_participants: usize,
        bridge: Arc<SignalBridge>,
    ) -> Self {
        let mut mux_writers = Vec::with_capacity(total_participants);
        let mut mux_readers = Vec::with_capacity(total_participants);

        for j in 0..total_participants {
            // Writer: Us -> J
            // Layout index: participant_index * P + j
            let writer_idx = participant_index * total_participants + j;
            let offset = writer_idx * region_size;
            let (header, data, data_len) =
                RingBufferHeader::from_raw_parts(base_ptr, offset, region_size);

            mux_writers.push(Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                header,
                data,
                data_len,
                bridge.clone(),
                j,
            ))));

            // Reader: J -> Us
            // Layout index: j * P + participant_index
            let reader_idx = j * total_participants + participant_index;
            let offset = reader_idx * region_size;
            let (header, data, data_len) =
                RingBufferHeader::from_raw_parts(base_ptr, offset, region_size);

            mux_readers.push(Arc::new(Mutex::new(MultiplexedDsmReader::new(
                header,
                data,
                data_len,
                bridge.clone(),
                j,
            ))));
        }

        Self {
            mux_writers,
            mux_readers,
            bridge,
        }
    }
}
