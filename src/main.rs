use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::collections::VecDeque;
use std::time::Instant;

use bluest::{Adapter, AdvertisingDevice};
use futures_lite::stream::StreamExt;
use tiny_http::{Server, Response, Header};
use serde::Serialize;

// 共享数据结构存储心率信息
struct HeartRateMonitor {
    current_rate: u8,
    device_name: String,
    rssi: i16,
    last_update: Instant,
    history: VecDeque<u8>,
}

#[derive(Serialize)]
struct HeartRateUpdate {
    heart_rate: u8,
    device_name: String,
    rssi: i16,
    elapsed_secs: u64,
    status: String,
    status_color: String,
    history: Vec<u8>,
}

impl HeartRateMonitor {
    fn new() -> Self {
        Self {
            current_rate: 0,
            device_name: "等待连接...".to_string(),
            rssi: i16::MIN,
            last_update: Instant::now(),
            history: VecDeque::with_capacity(60),
        }
    }
    
    fn update(&mut self, rate: u8, name: &str, rssi: i16) {
        self.current_rate = rate;
        self.device_name = name.to_string();
        self.rssi = rssi;
        self.last_update = Instant::now();
        
        // 更新历史数据
        self.history.push_back(rate);
        if self.history.len() > 60 {
            self.history.pop_front();
        }
    }
    
    fn is_recent(&self) -> bool {
        self.last_update.elapsed().as_secs() < 10
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 创建共享心率监视器
    let heart_rate = Arc::new(Mutex::new(HeartRateMonitor::new()));
    
    // 启动HTTP服务器线程
    let hr_web = heart_rate.clone();
    thread::spawn(move || {
        start_http_server(hr_web);
    });

    let adapter = Adapter::default()
        .await
        .ok_or("蓝牙设备未找到...")?;
    adapter.wait_available().await?;

    println!("开始扫描在线的小米设备...");
    println!("请访问: http://localhost:8080/ 查看心率监测");
    
    let mut scan = adapter.scan(&[]).await?;

    while let Some(discovered_device) = scan.next().await {
        // 使用闭包捕获共享心率监视器
        let hr_monitor = heart_rate.clone();
        handle_device(discovered_device, move |rate, name, rssi| {
            let mut monitor = hr_monitor.lock().unwrap();
            monitor.update(rate, name, rssi);
        });
    }
    Ok(())
}

// 原始处理函数保持不变，添加回调参数
fn handle_device<F>(discovered_device: AdvertisingDevice, callback: F) 
where
    F: FnOnce(u8, &str, i16),
{
    if let Some(manufacturer_data) = discovered_device.adv_data.manufacturer_data {
        if manufacturer_data.company_id != 0x0157 {
            return;
        }
        let name = discovered_device
            .device
            .name()
            .unwrap_or(String::from("(未知)"));
        if name != "Mi Smart Band 4" {
            return;
        }
        let rssi = discovered_device.rssi.unwrap_or_default();
        let heart_rate = manufacturer_data.data[3];
        println!("{name} ({rssi}dBm) 心率: {heart_rate:?}");
        
        // 调用回调函数更新共享状态
        callback(heart_rate, &name, rssi);
    }
}

// 创建心率更新数据结构
fn create_heart_rate_update(monitor: &HeartRateMonitor) -> HeartRateUpdate {
    let elapsed_secs = monitor.last_update.elapsed().as_secs();
    let status = if monitor.is_recent() { 
        "实时更新中".to_string() 
    } else { 
        "信号丢失".to_string() 
    };
    
    let status_color = if monitor.is_recent() { 
        "#27ae60".to_string() 
    } else { 
        "#e74c3c".to_string() 
    };
    
    HeartRateUpdate {
        heart_rate: monitor.current_rate,
        device_name: monitor.device_name.clone(),
        rssi: monitor.rssi,
        elapsed_secs,
        status,
        status_color,
        history: monitor.history.iter().cloned().collect(),
    }
}

// HTTP服务器实现
fn start_http_server(heart_rate: Arc<Mutex<HeartRateMonitor>>) {
    let addr = "0.0.0.0:8080";
    let server = Server::http(addr).expect("无法启动HTTP服务器");
    
    // 创建HTML内容类型头
    let html_content_type = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
        .expect("创建内容类型头失败");
    
    // 创建JSON内容类型头
    let json_content_type = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("创建内容类型头失败");

    for request in server.incoming_requests() {
        // 处理数据端点
        if request.url() == "/data" {
            let monitor = heart_rate.lock().unwrap();
            let update = create_heart_rate_update(&monitor);
            let json = serde_json::to_string(&update).unwrap();
            
            let response = Response::from_string(json)
                .with_header(json_content_type.clone());
            
            request.respond(response).expect("响应请求失败");
            continue;
        }
        
        // 主页面
        let html = r#"
        <!DOCTYPE html>
        <html lang="zh-CN">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>小米手环4 心率监测</title>
            <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
            <style>
                * {
                    box-sizing: border-box;
                    margin: 0;
                    padding: 0;
                }
                body {
                    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                    background: linear-gradient(135deg, #1a2a6c, #b21f1f, #fdbb2d);
                    color: #fff;
                    min-height: 100vh;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    padding: 20px;
                }
                .container {
                    background: rgba(30, 30, 46, 0.85);
                    backdrop-filter: blur(10px);
                    border-radius: 20px;
                    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
                    width: 100%;
                    max-width: 900px;
                    padding: 30px;
                    position: relative;
                    overflow: hidden;
                }
                .container::before {
                    content: '';
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    height: 5px;
                    background: linear-gradient(90deg, #ff8a00, #da1b60);
                }
                h1 {
                    text-align: center;
                    margin-bottom: 25px;
                    font-size: 32px;
                    color: #ffffff;
                    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
                }
                .status-header {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 30px;
                    flex-wrap: wrap;
                    gap: 20px;
                }
                .heart-rate-display {
                    font-size: 120px;
                    font-weight: 700;
                    color: #ff6b6b;
                    text-align: center;
                    text-shadow: 0 0 20px rgba(255, 107, 107, 0.7);
                    flex: 1;
                    min-width: 200px;
                }
                .bpm {
                    font-size: 24px;
                    color: #a9a9a9;
                    margin-top: -20px;
                }
                .device-info {
                    background: rgba(255, 255, 255, 0.1);
                    border-radius: 15px;
                    padding: 20px;
                    flex: 1;
                    min-width: 250px;
                }
                .info-item {
                    margin-bottom: 15px;
                    display: flex;
                    justify-content: space-between;
                }
                .info-label {
                    color: #ccc;
                }
                .info-value {
                    font-weight: 600;
                    color: #fff;
                }
                .chart-container {
                    background: rgba(255, 255, 255, 0.1);
                    border-radius: 15px;
                    padding: 20px;
                    height: 350px;
                    margin-top: 20px;
                }
                .footer {
                    margin-top: 25px;
                    text-align: center;
                    font-size: 14px;
                    color: #aaa;
                }
                @media (max-width: 768px) {
                    .status-header {
                        flex-direction: column;
                    }
                    .heart-rate-display {
                        font-size: 80px;
                    }
                }
            </style>
        </head>
        <body>
            <div class="container">
                <h1>小米手环4 实时心率监测</h1>
                
                <div class="status-header">
                    <div class="heart-rate-display">
                        <span id="heart-rate">0</span>
                        <div class="bpm">BPM</div>
                    </div>
                    
                    <div class="device-info">
                        <div class="info-item">
                            <span class="info-label">设备名称:</span>
                            <span class="info-value" id="device-name">未知</span>
                        </div>
                        <div class="info-item">
                            <span class="info-label">信号强度:</span>
                            <span class="info-value" id="rssi">- dBm</span>
                        </div>
                        <div class="info-item">
                            <span class="info-label">最后更新:</span>
                            <span class="info-value" id="last-update">- 秒前</span>
                        </div>
                        <div class="info-item">
                            <span class="info-label">当前状态:</span>
                            <span class="info-value" id="status">等待数据...</span>
                        </div>
                    </div>
                </div>
                
                <div class="chart-container">
                    <canvas id="heartRateChart"></canvas>
                </div>
                
                <div class="footer">
                    实时数据更新 | 小米手环4心率监测系统
                </div>
            </div>

            <script>
                // 初始化图表
                function initChart() {
                    const ctx = document.getElementById('heartRateChart').getContext('2d');
                    window.heartRateChart = new Chart(ctx, {
                        type: 'line',
                        data: {
                            datasets: [{
                                label: '心率 (BPM)',
                                data: [],
                                borderColor: '#ff6b6b',
                                backgroundColor: 'rgba(255, 107, 107, 0.1)',
                                borderWidth: 3,
                                pointRadius: 4,
                                pointBackgroundColor: '#fff',
                                pointBorderColor: '#ff6b6b',
                                tension: 0.4,
                                fill: true
                            }]
                        },
                        options: {
                            responsive: true,
                            maintainAspectRatio: false,
                            animation: {
                                duration: 0
                            },
                            scales: {
                                x: {
                                    type: 'linear', // 添加这行
                                    min: 0,         // 添加这行
                                    max: 59,        // 添加这行
                                    grid: {
                                        color: 'rgba(255, 255, 255, 0.1)'
                                    },
                                    ticks: {
                                        color: '#ccc'
                                    },
                                    title: {
                                        display: true,
                                        text: '时间 (最近60个读数)',
                                        color: '#ccc'
                                    }
                                },
                                y: {
                                    min: 40,
                                    max: 120,
                                    grid: {
                                        color: 'rgba(255, 255, 255, 0.1)'
                                    },
                                    ticks: {
                                        color: '#ccc',
                                        stepSize: 20
                                    },
                                    title: {
                                        display: true,
                                        text: '心率 (BPM)',
                                        color: '#ccc'
                                    }
                                }
                            },
                            plugins: {
                                legend: {
                                    labels: {
                                        color: '#ccc'
                                    }
                                },
                                // 添加以下工具提示配置
                                tooltip: {
                                    callbacks: {
                                        title: (items) => `读数 #${items[0].parsed.x}`,
                                        label: (context) => `心率: ${context.parsed.y} BPM`
                                    }
                                }
                            }
                        }
                    });
                }
                
                // 更新UI函数
                function updateUI(data) {
                    // 更新心率显示
                    document.getElementById('heart-rate').textContent = data.heart_rate;
                    
                    // 更新设备名称
                    document.getElementById('device-name').textContent = data.device_name;
                    
                    // 更新RSSI
                    document.getElementById('rssi').textContent = data.rssi + ' dBm';
                    
                    // 更新最后更新时间
                    document.getElementById('last-update').textContent = data.elapsed_secs + '秒前';
                    
                    // 更新状态
                    const statusElement = document.getElementById('status');
                    statusElement.textContent = data.status;
                    statusElement.style.color = data.status_color;
                    
                    // 更新图表
                    updateChart(data.history);
                }
                
                // 更新图表
                function updateChart(history) {
                    if (!window.heartRateChart) {
                        initChart();
                    }
                    
                    // 创建正确的数据点数组
                    const newData = [];
                    const startIndex = Math.max(0, history.length - 60);
                    
                    for (let i = startIndex; i < history.length; i++) {
                        // 计算正确的x轴位置（从0到59）
                        const x = 59 - (history.length - 1 - i);
                        newData.push({ 
                            x: x, 
                            y: history[i]
                        });
                    }
                    
                    // 如果数据不足60个，在前面填充空点
                    if (newData.length < 60) {
                        const emptyPoints = 60 - newData.length;
                        for (let i = 0; i < emptyPoints; i++) {
                            newData.unshift({ x: i, y: null });
                        }
                    }
                    
                    window.heartRateChart.data.datasets[0].data = newData;
                    window.heartRateChart.update('none');
                }
                
                // 获取最新心率数据
                async function fetchHeartRateData() {
                    try {
                        const response = await fetch('/data');
                        if (!response.ok) {
                            throw new Error('网络响应异常');
                        }
                        const data = await response.json();
                        updateUI(data);
                    } catch (error) {
                        console.error('获取数据失败:', error);
                        document.getElementById('status').textContent = '数据获取失败';
                    }
                }
                
                // 初始化图表
                initChart();
                
                // 立即获取数据
                fetchHeartRateData();
                
                // 每2秒获取一次数据
                setInterval(fetchHeartRateData, 2000);
            </script>
        </body>
        </html>
        "#;
        
        // 创建响应并设置正确的Content-Type头
        let response = Response::from_string(html)
            .with_header(html_content_type.clone());
        
        request.respond(response).expect("响应请求失败");
    }
}