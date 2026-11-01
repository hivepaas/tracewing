//! Whole bodies on disk, for the ones too big to wait for in memory.
//!
//! Like flows, bodies go to one writer thread and the proxy never waits on the
//! disk. The frames queued are the same buffers the client is being sent, not
//! copies. A byte budget bounds them: when the disk cannot keep up, a body over
//! the budget stops being kept, and says so. So does one past the most a body may
//! keep, and so does any body when the disk is nearly full. Traffic is never held
//! back, and a body with a hole in it is never presented as whole.

use crate::session::Shared;
use anyhow::{Context, Result};
use bytes::Bytes;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::JoinHandle,
};

const WRITE_BUFFER: usize = 256 * 1024;

#[derive(Debug, Default)]
pub struct SpoolStats {
    pub bytes: AtomicU64,
    pub files: AtomicU64,
    /// Bodies that stopped being kept because the queue was over budget.
    pub lost: AtomicU64,
    pub write_errors: AtomicU64,
    /// The most ever queued, in bytes.
    pub peak_queued: AtomicU64,
}

enum Message {
    Data {
        file: u64,
        data: Bytes,
    },
    End {
        file: u64,
    },
    /// In this queue because the file may not exist yet: deleted from anywhere
    /// else, it would be made afterwards and never deleted.
    Remove {
        name: String,
    },
    Barrier(Sender<()>),
    Close,
}

#[derive(Clone)]
pub struct Spool {
    tx: Sender<Message>,
    dir: PathBuf,
    next: Arc<AtomicU64>,
    queued: Arc<AtomicU64>,
    budget: u64,
    body_max: u64,
    shared: Arc<Shared>,
    stats: Arc<SpoolStats>,
}

impl Spool {
    pub fn spawn(
        dir: &Path,
        budget: u64,
        body_max: u64,
        shared: Arc<Shared>,
    ) -> Result<(Self, SpoolThread)> {
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        let (tx, rx) = mpsc::channel();
        let dir: PathBuf = dir.to_owned();
        let queued = Arc::new(AtomicU64::new(0));
        let stats = Arc::new(SpoolStats::default());
        let handle = {
            let (dir, queued, stats) = (dir.clone(), Arc::clone(&queued), Arc::clone(&stats));
            std::thread::Builder::new()
                .name("body-spool".into())
                .spawn(move || write_loop(&dir, rx, &queued, &stats))?
        };
        let thread = SpoolThread {
            handle,
            tx: tx.clone(),
        };
        Ok((
            Self {
                tx,
                dir: dir.clone(),
                next: Arc::new(AtomicU64::new(0)),
                queued,
                budget,
                body_max,
                shared,
                stats,
            },
            thread,
        ))
    }

    pub fn stats(&self) -> Arc<SpoolStats> {
        Arc::clone(&self.stats)
    }

    /// Deletes a file once everything already queued has been dealt with.
    pub fn remove(&self, name: String) {
        if let Err(mpsc::SendError(Message::Remove { name })) =
            self.tx.send(Message::Remove { name })
        {
            // The thread has gone, and everything it was given with it.
            let _ = std::fs::remove_file(self.dir.join(name));
        }
    }

    /// Waits until everything queued so far is on the disk.
    pub fn barrier(&self) {
        let (done, wait) = mpsc::channel();
        if self.tx.send(Message::Barrier(done)).is_ok() {
            let _ = wait.recv();
        }
    }

    pub fn begin(&self) -> SpoolFile {
        SpoolFile {
            spool: self.clone(),
            id: self.next.fetch_add(1, Ordering::Relaxed),
            kept: 0,
            lost: false,
        }
    }
}

/// One body on its way to disk. Ends when dropped.
pub struct SpoolFile {
    spool: Spool,
    id: u64,
    kept: u64,
    lost: bool,
}

impl SpoolFile {
    pub fn name(&self) -> String {
        file_name(self.id)
    }

    pub fn is_lost(&self) -> bool {
        self.lost
    }

    /// Bytes sent to the disk so far.
    pub fn kept(&self) -> u64 {
        self.kept
    }

    pub fn write(&mut self, data: Bytes) {
        if self.lost {
            return;
        }
        let len = data.len() as u64;
        let queued = self.spool.queued.fetch_add(len, Ordering::Relaxed) + len;
        if queued > self.spool.budget
            || self.kept + len > self.spool.body_max
            || self.spool.shared.low_disk.load(Ordering::Relaxed)
        {
            self.spool.queued.fetch_sub(len, Ordering::Relaxed);
            self.lost = true;
            self.spool.stats.lost.fetch_add(1, Ordering::Relaxed);
            return;
        }
        self.spool
            .stats
            .peak_queued
            .fetch_max(queued, Ordering::Relaxed);
        self.kept += len;
        let _ = self.spool.tx.send(Message::Data {
            file: self.id,
            data,
        });
    }
}

impl Drop for SpoolFile {
    fn drop(&mut self) {
        let _ = self.spool.tx.send(Message::End { file: self.id });
    }
}

pub struct SpoolThread {
    handle: JoinHandle<()>,
    tx: Sender<Message>,
}

impl SpoolThread {
    /// Writes everything queued and stops, like the flow writer and for the
    /// same reason: tunnels still open keep a `Spool` alive.
    pub fn close(self) -> Result<()> {
        let _ = self.tx.send(Message::Close);
        self.handle
            .join()
            .map_err(|_| anyhow::anyhow!("spool writer panicked"))
    }
}

fn file_name(id: u64) -> String {
    format!("{id}.body")
}

fn write_loop(dir: &Path, rx: Receiver<Message>, queued: &AtomicU64, stats: &SpoolStats) {
    let mut open: HashMap<u64, Option<std::io::BufWriter<File>>> = HashMap::new();
    while let Ok(message) = rx.recv() {
        match message {
            Message::Data { file, data } => {
                let writer = open.entry(file).or_insert_with(|| {
                    stats.files.fetch_add(1, Ordering::Relaxed);
                    match File::create(dir.join(file_name(file))) {
                        Ok(created) => {
                            Some(std::io::BufWriter::with_capacity(WRITE_BUFFER, created))
                        }
                        Err(_) => {
                            stats.write_errors.fetch_add(1, Ordering::Relaxed);
                            None
                        }
                    }
                });
                if let Some(out) = writer {
                    if out.write_all(&data).is_ok() {
                        stats.bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
                    } else {
                        // A full disk: the rest of this body is gone, not the proxy.
                        stats.write_errors.fetch_add(1, Ordering::Relaxed);
                        *writer = None;
                    }
                }
                queued.fetch_sub(data.len() as u64, Ordering::Relaxed);
            }
            Message::End { file } => {
                if let Some(Some(mut out)) = open.remove(&file)
                    && out.flush().is_err()
                {
                    stats.write_errors.fetch_add(1, Ordering::Relaxed);
                }
            }
            Message::Remove { name } => {
                let _ = std::fs::remove_file(dir.join(name));
            }
            Message::Barrier(done) => {
                // What has ended is flushed; what has not is still being written.
                let _ = done.send(());
            }
            Message::Close => break,
        }
    }
    for (_, writer) in open {
        if let Some(mut out) = writer
            && out.flush().is_err()
        {
            stats.write_errors.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_removed_right_after_its_body_stays_removed() {
        let dir = std::env::temp_dir().join(format!("spool-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (spool, thread) =
            Spool::spawn(&dir, u64::MAX, u64::MAX, Arc::new(Shared::default())).unwrap();
        // However far behind the thread is, these are done in this order.
        let mut names = Vec::new();
        for _ in 0..100 {
            let mut file = spool.begin();
            file.write(Bytes::from_static(b"some of a body"));
            names.push(file.name());
        }
        for name in names {
            spool.remove(name);
        }
        spool.barrier();
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        thread.close().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_barrier_comes_after_what_has_ended() {
        let dir = std::env::temp_dir().join(format!("spool-b-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (spool, thread) =
            Spool::spawn(&dir, u64::MAX, u64::MAX, Arc::new(Shared::default())).unwrap();
        let mut file = spool.begin();
        let name = file.name();
        file.write(Bytes::from_static(b"a whole body"));
        drop(file);
        spool.barrier();
        assert_eq!(std::fs::read(dir.join(name)).unwrap(), b"a whole body");
        thread.close().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
