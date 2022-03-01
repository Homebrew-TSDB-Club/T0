use crate::chunk::{MutableChunk, ScanChunk};
use crate::error::{ScanError, WriteError};
use common::time::{Instant, EPOCH};
use common::{Label, Scalar};
use context::Schema;
use ql::rosetta::{Matcher, Range};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct Table {
    name: Arc<str>,
    mutable_chunks: VecDeque<MutableChunk>,

    schema: Arc<Schema>,
}

impl Table {
    pub(crate) fn new(name: Arc<str>, schema: Arc<Schema>) -> Self {
        let mutable_chunks = VecDeque::new();
        Self {
            name,
            mutable_chunks,
            schema,
        }
    }

    pub(crate) fn write(
        &mut self,
        labels: &[Label],
        scalars: &[(Instant, Vec<Scalar>)],
    ) -> Result<(), WriteError> {
        for (timestamp, scalars) in scalars {
            let chunk = self.lookup_mutable_chunk(*timestamp)?;
            let aligned_labels = chunk.align_labels(labels);
            let mut row = match chunk.get_mut(&aligned_labels)? {
                Some(row) => row,
                None => chunk.push(&aligned_labels),
            };
            row.insert(*timestamp, scalars);
        }
        Ok(())
    }

    pub(crate) async fn scan(
        &self,
        projections: Option<&[String]>,
        filters: &[Matcher<'_>],
        range: Range,
        limit: Option<usize>,
    ) -> Result<Vec<ScanChunk>, ScanError> {
        let mut chunks = Vec::new();
        let mut tasks = Vec::new();
        for chunk in &self.mutable_chunks {
            if let Some(start) = range.start {
                if chunk.info.end_at() < start {
                    continue;
                }
            }
            if let Some(end) = range.end {
                if chunk.info.start_at > end {
                    continue;
                }
            }
            tasks.push(runtime::spawn(chunk.scan(
                projections,
                filters,
                range,
                limit,
            )));
        }
        for task in tasks {
            if let Some(chunk) = task.await? {
                chunks.push(chunk);
            }
        }
        Ok(chunks)
    }

    fn lookup_mutable_chunk(
        &mut self,
        timestamp: Instant,
    ) -> Result<&mut MutableChunk, WriteError> {
        match self.mutable_chunks.front() {
            Some(first_chunk) => {
                if timestamp < first_chunk.info.start_at {
                    return Err(WriteError::TimestampArchived {
                        t: timestamp,
                        table_name: Arc::clone(&self.name),
                    });
                }
                let mut offset =
                    (timestamp - first_chunk.info.start_at) / self.schema.meta.chunk_duration();
                let delta = offset - (self.mutable_chunks.len() as i64 - 1);
                if delta > 0 {
                    for i in 0..delta {
                        self.mutable_chunks.push_back(
                            self.new_mutable_chunk(timestamp - self.schema.meta.time_interval * i),
                        );
                    }
                }
                let delta =
                    self.mutable_chunks.len() as i64 - self.schema.meta.mutable_chunk_num as i64;
                if delta > 0 {
                    for _ in 0..delta {
                        let chunk = self.mutable_chunks.pop_front().unwrap();
                        runtime::spawn(chunk.archive()).detach();
                    }
                    offset -= delta;
                }
                Ok(&mut self.mutable_chunks[offset as usize])
            }
            None => {
                self.mutable_chunks
                    .push_back(self.new_mutable_chunk(timestamp));
                Ok(self.mutable_chunks.back_mut().unwrap())
            }
        }
    }

    fn new_mutable_chunk(&self, timestamp: Instant) -> MutableChunk {
        let start_at = EPOCH
            + (self.schema.meta.chunk_duration() * (timestamp / self.schema.meta.chunk_duration()));

        MutableChunk::new(Arc::clone(&self.schema), start_at)
    }
}
