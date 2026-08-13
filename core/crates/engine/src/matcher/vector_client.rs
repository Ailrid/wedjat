/*
 * @Author: ShirahaYuki  shirhayuki2002@gmail.com
 * @Date: 2026-01-14 10:10:14
 * @LastEditors: ShirahaYuki  shirhayuki2002@gmail.com
 * @LastEditTime: 2026-01-19 18:55:33
 * @FilePath: /map_matching/src/matcher/vector_client.rs
 * @Description:负责与图像/向量数据库进行交互，发送查询请求和接收响应。
 *
 * Copyright (c) 2026 by ShirahaYuki, All Rights Reserved.
 */
use crate::matcher::errors::ClientError;
use crate::matcher::types::{CropRequest, SearchRequest, SearchResponse};
use opencv::{core, imgcodecs};
use reqwest::blocking::Client;

pub struct VectorClient {
    client: Client,
    addr: String,
}

impl VectorClient {
    pub fn new(addr: String) -> Self {
        let client = Client::new();
        Self { client, addr }
    }
    /// Describe this function.
    ///
    /// # Arguments
    ///
    /// - `request` (`SearchRequest`) - 请求数据结构体.
    ///
    /// # Returns
    ///
    /// - `Result<SearchResponse, SearchError>` - 返回查询.
    ///

    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse, ClientError> {
        let url = format!("{}/search", self.addr);
        let response = self
            .client
            .post(url)
            .json(&request)
            .send()?
            .json::<SearchResponse>()?;
        Ok(response)
    }
    /// Describe this function.
    ///
    /// # Arguments
    ///
    /// - `request` (`CropRequest`) -  裁剪请求结构体
    ///
    /// # Returns
    ///
    /// - `Result<core::Mat, CropError>` - 转换完成的opencv Mat结构体

    pub fn crop(&self, request: CropRequest) -> Result<core::Mat, ClientError> {
        let url = format!("{}/crop", self.addr);
        let response = self.client.post(url).json(&request).send()?.bytes()?;
        let buf = core::Vector::<u8>::from_iter(response.as_ref().iter().cloned());
        //转换字节为Mat
        let mat = imgcodecs::imdecode(&buf, imgcodecs::IMREAD_COLOR)?;

        Ok(mat)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::matcher::types::SearchItem;
//     use opencv::imgcodecs;

//     #[test]
//     fn test_image_crop_and_save() {
//         // 实例化客户端，地址指向你的 Python FastAPI 服务器
//         let client = VectorClient::new("http://127.0.0.1:8000".to_string());

//         // 构造一个模拟的 SearchItem 作为 payload 传入
//         let dummy_item = SearchItem {
//             score: 1.0,
//             utm_x: 0.0,
//             utm_y: 0.0,
//             utm_zone: 50,
//             crs: "EPSG:32650".to_string(),
//             pixel_x: 0,
//             pixel_y: 0,
//             res: vec![0.5, 0.5],
//             src: "merged_lv18_utm.tif".to_string(),
//         };

//         // 构造裁剪请求，坐标固定为 0
//         let request = CropRequest {
//             pixel_x: 0,
//             pixel_y: 0,
//             patch_size: 256,
//             src: "merged_lv18_utm.tif".to_string(),
//         };

//         // 执行请求并获取 Mat
//         let result = client.crop(request);

//         match result {
//             Ok(mat) => {
//                 // 定义保存路径
//                 let save_path = "crop_result.png";

//                 // 使用 opencv 库保存图像
//                 let params = core::Vector::<i32>::new();
//                 let write_res = imgcodecs::imwrite(save_path, &mat, &params);

//                 match write_res {
//                     Ok(success) => {
//                         if success {
//                             println!("✅ 图像已成功保存至: {}", save_path);
//                         } else {
//                             panic!("❌ OpenCV imwrite 返回失败");
//                         }
//                     }
//                     Err(e) => panic!("❌ 保存图像时出错: {:?}", e),
//                 }
//             }
//             Err(e) => {
//                 panic!("❌ Crop 请求失败: {:?}", e);
//             }
//         }
//     }
// }
