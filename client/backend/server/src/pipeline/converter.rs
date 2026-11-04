use super::error::ConverterError;
use super::fns::*;
use crate::config::Config;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy)]
pub enum ConvertType {
    Las2Tiles,
    Osgb2Tiles,
    Tiff2Mbtiles,
    Tiff2Terrain,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub source_path: String,
    pub group_name: Option<String>,
    pub epsg: i32,
    pub convert_type: ConvertType,
}

pub struct Worker {
    tx: mpsc::Sender<Task>,
}

impl Worker {
    fn new(config: Arc<Config>, pool: PgPool, capacity: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<Task>(capacity);

        let cfg = config;
        let db = pool;

        tokio::spawn(async move {
            while let Some(task) = rx.recv().await {
                match task.convert_type {
                    ConvertType::Las2Tiles => las2tiles(task, cfg.clone(), db.clone()).await,
                    ConvertType::Osgb2Tiles => {
                        osgb2tiles(task, cfg.clone(), db.clone()).await;
                    }
                    ConvertType::Tiff2Mbtiles => {
                        tiff2mbtiles(task, cfg.clone(), db.clone()).await;
                    }
                    ConvertType::Tiff2Terrain => {
                        tiff2terrain(task, cfg.clone(), db.clone()).await;
                    }
                }
            }
        });

        Worker { tx }
    }

    fn load(&self) -> usize {
        self.tx.max_capacity() - self.tx.capacity()
    }

    fn remaining(&self) -> usize {
        self.tx.capacity()
    }

    fn try_submit(&self, task: Task) -> Result<(), ConverterError> {
        let _ = self.tx.try_send(task)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Converter {
    workers: Arc<Vec<Worker>>,
}

impl Converter {
    pub fn new(config: Arc<Config>, pool: PgPool) -> Self {
        let n = config.conversion.max_converters.max(1) as usize;
        let workers = (0..n)
            .map(|_| {
                Worker::new(
                    config.clone(),
                    pool.clone(),
                    config.conversion.queue_capacity,
                )
            })
            .collect();
        Converter {
            workers: Arc::new(workers),
        }
    }

    pub fn convert(&self, task: Task) -> Result<(), ConverterError> {
        let mut order: Vec<usize> = (0..self.workers.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(self.workers[i].remaining()));

        self.workers[0].try_submit(task)?;

        Err(ConverterError::QueueFull)
    }

    pub fn total_load(&self) -> usize {
        self.workers.iter().map(|w| w.load()).sum()
    }
}
