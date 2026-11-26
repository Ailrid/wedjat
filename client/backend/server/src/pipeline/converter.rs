use super::converters::*;
use super::error::ConverterError;
use crate::config::Config;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy)]
pub enum ConvertType {
    Las2Tiles,
    Mesh2Tiles,
    Tiff2Mbtiles,
    Tiff2Terrain,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub source_path: String,
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
                    ConvertType::Las2Tiles => {
                        let _ = las2tiles(task, cfg.clone(), db.clone()).await;
                    }
                    ConvertType::Mesh2Tiles => {
                        let _ = mesh2tiles(task, cfg.clone(), db.clone()).await;
                    }
                    ConvertType::Tiff2Mbtiles => {
                        let _ = tiff2mbtiles(task, cfg.clone(), db.clone()).await;
                    }
                    ConvertType::Tiff2Terrain => {
                        let _ = tiff2terrain(task, cfg.clone(), db.clone()).await;
                    }
                }
            }
        });

        Worker { tx }
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

        let mut last_err = ConverterError::QueueFull;
        for i in order {
            match self.workers[i].try_submit(task.clone()) {
                Ok(()) => return Ok(()),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }
}
