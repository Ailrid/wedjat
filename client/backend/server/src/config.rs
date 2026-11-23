use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DataConfig {
    pub output_root: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub directory: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MbtilesConfig {
    pub threads: usize,    // 0 = ALL_CPUS，透传给 gdalwarp -wo NUM_THREADS
    pub timeout_secs: u64, // 单条 gdal 命令的超时
    pub min_zoom: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TerrainConfig {
    pub java_bin: String,
    pub jar_path: String,
    pub max_heap: String,
    pub max_depth: u8,
    pub nodata_value: f64,
    pub geoid: String,
    pub raster_max_size: u32, // 2048 省内存 / 4096 快
    pub attribution: String,  // 写进 layer.json，可以是空串
    pub timeout_secs: u64,
}
#[derive(Debug, Deserialize, Clone)]
pub struct PointCloudConfig {
    pub bin_path: String, // gotiler 可执行文件（同目录必须有 share/ PROJ 数据）
    pub timeout_secs: u64,
    pub points_per_tile: u32, // 唯一的质量/体积旋钮
    pub colorize: String,     // "" = 保留原始 RGB；否则 "z:viridis" / "intensity:turbo"
}
#[derive(Debug, Deserialize, Clone)]
pub struct MeshConfig {
    pub java_bin: String,
    pub jar_path: String, // mago-3d-tiler.jar
    pub max_heap: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConversionConfig {
    pub max_converters: usize,
    pub queue_capacity: usize,
    pub mbtiles: MbtilesConfig,
    pub terrain: TerrainConfig,
    pub pointcloud: PointCloudConfig,
    pub mesh: MeshConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub conversion: ConversionConfig,
    pub data: DataConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, config::ConfigError> {
        let builder =
            config::Config::builder().add_source(config::File::with_name(path).required(true));

        let settings = builder.build()?;
        settings.try_deserialize()
    }
}
