export type ImageryType = "xyz" | "wms" | "wmts";

export interface XyzConfig {
  type: "xyz";
  credit: string;
  subdomains: string;
  minimumLevel: number;
  maximumLevel: number;
  tilingScheme: "WebMercator" | "Geographic";
}

export interface WmsConfig {
  type: "wms";
  credit: string;
  layers: string;
  version: "1.1.1" | "1.3.0";
  format: string;
  enablePickFeatures?: boolean;
  minimumLevel: number;
  maximumLevel: number;
  tilingScheme: "WebMercator" | "Geographic";
}

export interface WmtsConfig {
  type: "wmts";
  credit: string;
  layer: string;
  style: string;
  tileMatrixSetID: string;
  format: string;
  subdomains: string;
  tileMatrixLabels: string;
  minimumLevel: number;
  maximumLevel: number;
  tilingScheme: "WebMercator" | "Geographic";
  projection: "c" | "w";
}

export type ImageryConfig = XyzConfig | WmsConfig | WmtsConfig;
// --------------------------------------------------------------------------

export interface TerrainConfig {
  requestVertexNormals: boolean;
  requestWaterMask: boolean;
  credit: string;
}

// --------------------------------------------------------------------------
export type VectorType = "geojson" | "kml";
export interface PointStyle {
  color: string;
  size: number;
  outlineColor: string;
  outlineWidth: number;
}

export interface PolylineStyle {
  color: string;
  width: number;
}

export interface PolygonStyle {
  color: string;
  outline: boolean;
  outlineColor: string;
  outlineWidth: number;
}

export interface VectorConfig {
  type: VectorType;
  clampToGround: boolean;
  point: PointStyle;
  polyline: PolylineStyle;
  polygon: PolygonStyle;
}
// --------------------------------------------------------------------------
export type PointCloudStyleMode =
  | "original"
  | "uniform"
  | "classification"
  | "elevation";

export interface ClassificationRule {
  classification: number;
  color: string;
  visible: boolean;
}

// 高程分段点（按 elevation 升序）
export interface ElevationStop {
  elevation: number; // 高程值（米）
  color: string; // hex
}

export interface PointCloudConfig {
  type: "pcd";
  styleMode: PointCloudStyleMode;

  // ---- uniform 模式 ----
  uniformColor: string;

  // ---- classification 模式 ----
  classificationRules: ClassificationRule[];

  // ---- elevation 模式 ----
  elevationStops: ElevationStop[];

  // ---- PointCloudShading ----
  attenuation: boolean;
  eyeDomeLighting: boolean;

  // ---- 通用 tileset ----
  maximumScreenSpaceError: number;
  dynamicScreenSpaceError: boolean;
  credit: string;
}

// ---------- OSGB 转换产物 ----------
export interface OsgbConfig {
  type: "osgb";
  /** 屏幕空间误差，默认 16；倾斜模型可适当放宽 */
  maximumScreenSpaceError: number;
  /** 阴影模式 */
  shadows: "ENABLED" | "DISABLED" | "RECEIVE_ONLY" | "CAST_ONLY";
  /** 动态屏幕空间误差，默认建议 true */
  dynamicScreenSpaceError: boolean;
  /** 最大内存占用（MB），默认 512 */
  maximumMemoryUsage: number;

  credit: string;
}

export type TilesConfig = OsgbConfig | PointCloudConfig;
// --------------------------------------------------------------------------
export type ResourceType =
  | "imagery" // DOM、专题底图（XYZ / WMS / WMTS）
  | "terrain" // DEM 切片后的 Cesium Terrain
  | "tiles" // 城市三维模型 / 激光雷达点云（入口 tileset.json）
  | "vector"; // GeoJSON / KML

export interface ResourceExtra {
  tiles?: TilesConfig;
  vector?: VectorConfig;
  terrain?: TerrainConfig;
  imagery?: ImageryConfig;
}

export interface Resource {
  id: string;
  show: boolean;
  alpha: number;
  type: ResourceType;
  url: string;
  name: string;
  extra: ResourceExtra;
  resource?: ResourceResponse;
}

// 内部资源
export interface ResourceResponse {
  id: number;
  name: string;
  category: string;
  url: string | null;
  status: string;
  error_message: string | null;
  min_lon: number | null;
  min_lat: number | null;
  max_lon: number | null;
  max_lat: number | null;
  min_zoom: number | null;
  max_zoom: number | null;
  epsg: number;
  group_name: string | null;
  extra: ResourceExtra;
  created_at: string;
  updated_at: string;
}
