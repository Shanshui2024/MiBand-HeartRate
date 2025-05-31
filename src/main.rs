use std::{error::Error, sync::{Arc, Mutex}, thread, time::Instant};
use bluest::{Adapter, AdvertisingDevice};
use futures_lite::stream::StreamExt;
use tiny_http::{Server, Response, Header};

// 心率监视器结构体
struct HeartRateMonitor {
    current_rate: u8,
    last_update: Instant,
}

impl HeartRateMonitor {
    fn new() -> Self {
        Self {
            current_rate: 0,
            last_update: Instant::now(),
        }
    }
    
    fn update(&mut self, rate: u8) {
        self.current_rate = rate;
        self.last_update = Instant::now();
    }
}

// 启动HTTP服务器
fn start_http_server(heart_rate: Arc<Mutex<HeartRateMonitor>>) {
    let addr = "0.0.0.0:1145";
    let server = Server::http(addr).expect("无法启动HTTP服务器");
    
    let html_content_type = Header::from_bytes("Content-Type", "text/html; charset=utf-8")
        .expect("创建内容类型头失败");
    
    let json_content_type = Header::from_bytes("Content-Type", "application/json")
        .expect("创建内容类型头失败");

    for request in server.incoming_requests() {
        // 数据端点
        if request.url() == "/data" {
            let monitor = heart_rate.lock().unwrap();
            let response = Response::from_string(format!("{}", monitor.current_rate))
                .with_header(json_content_type.clone());
            request.respond(response).expect("响应请求失败");
            continue;
        }
        
        // 根路径 - 扁平化霓虹灯管风格心率显示 (900x550)
        let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>霓虹灯管心率监测</title>
            <style>
                * {
                    margin: 0;
                    padding: 0;
                    box-sizing: border-box;
                }
                
                body {
                    background: #0a0a20;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    height: 100vh;
                    overflow: hidden;
                    font-family: 'Rajdhani', sans-serif;
                }
                
                .neon-container {
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    gap: 60px;
                    padding: 40px;
                    background: rgba(10, 10, 30, 0.8);
                    border-radius: 20px;
                    box-shadow: 0 0 50px rgba(0, 100, 255, 0.4);
                    position: relative;
                    overflow: hidden;
                    width: 900px;
                    height: 550px; /* 900x550尺寸 */
                }
                
                /* 霓虹灯管效果 */
                .neon-container::before {
                    content: '';
                    position: absolute;
                    top: 0;
                    left: 0;
                    right: 0;
                    height: 3px;
                    background: linear-gradient(90deg, transparent, #00ffff, transparent);
                    animation: scan 3s linear infinite;
                }
                
                .heart-section {
                    display: flex;
                    flex-direction: column;
                    align-items: center;
                }
                
                .neon-heart {
                    font-size: 240px; /* 更大的爱心 */
                    color: #ff0000;
                    text-shadow: 
                        0 0 20px #ff0000,
                        0 0 40px #ff0000,
                        0 0 80px #ff0000;
                    animation: heartbeat 1s infinite;
                    position: relative;
                    line-height: 1;
                    margin-bottom: 30px;
                }
                
                .heart-label {
                    font-size: 36px;
                    color: #ff5050;
                    text-shadow: 0 0 15px #ff0000;
                    letter-spacing: 3px;
                }
                
                .rate-section {
                    display: flex;
                    flex-direction: column;
                    align-items: center;
                }
                
                .neon-rate {
                    font-size: 240px; /* 更大的数字 */
                    font-weight: bold;
                    color: #0066ff;
                    text-shadow: 
                        0 0 20px #0066ff,
                        0 0 40px #0066ff,
                        0 0 80px #0066ff;
                    position: relative;
                    line-height: 1;
                }
                
                .neon-rate::after {
                    content: attr(data-rate);
                    position: absolute;
                    top: 0;
                    left: 0;
                    color: #00ffff;
                    filter: blur(25px);
                    z-index: -1;
                }
                
                .bpm {
                    font-size: 50px;
                    color: #00ffff;
                    text-shadow: 0 0 15px #00ffff;
                    margin-top: 20px;
                    letter-spacing: 5px;
                }
                
                .status-dot {
                    position: absolute;
                    bottom: 30px;
                    right: 30px;
                    width: 20px;
                    height: 20px;
                    background: #ff0000;
                    border-radius: 50%;
                    box-shadow: 0 0 15px #ff0000;
                }
                
                .connection-status {
                    position: absolute;
                    bottom: 30px;
                    left: 30px;
                    color: #00ffff;
                    font-size: 24px;
                    text-shadow: 0 0 8px #00ffff;
                }
                
                /* 霓虹灯边框效果 */
                .neon-border {
                    position: absolute;
                    top: 0;
                    left: 0;
                    width: 100%;
                    height: 100%;
                    border: 3px solid #00ffff;
                    box-shadow: 
                        inset 0 0 20px #00ffff,
                        0 0 20px #00ffff;
                    border-radius: 20px;
                    pointer-events: none;
                }
                
                /* 霓虹灯角落装饰 */
                .corner {
                    position: absolute;
                    width: 30px;
                    height: 30px;
                }
                
                .corner-tl {
                    top: -1px;
                    left: -1px;
                    border-top: 5px solid #ff00ff;
                    border-left: 5px solid #ff00ff;
                }
                
                .corner-tr {
                    top: -1px;
                    right: -1px;
                    border-top: 5px solid #ff00ff;
                    border-right: 5px solid #ff00ff;
                }
                
                .corner-bl {
                    bottom: -1px;
                    left: -1px;
                    border-bottom: 5px solid #ff00ff;
                    border-left: 5px solid #ff00ff;
                }
                
                .corner-br {
                    bottom: -1px;
                    right: -1px;
                    border-bottom: 5px solid #ff00ff;
                    border-right: 5px solid #ff00ff;
                }
                
                /* 动画效果 */
                @keyframes heartbeat {
                    0% { transform: scale(1); }
                    15% { transform: scale(1.15); }
                    30% { transform: scale(1); }
                    45% { transform: scale(1.1); }
                    60% { transform: scale(1); }
                    100% { transform: scale(1); }
                }
                
                @keyframes scan {
                    0% { transform: translateX(-100%); }
                    100% { transform: translateX(100%); }
                }
                
                .pulse {
                    position: absolute;
                    top: 50%;
                    left: 50%;
                    transform: translate(-50%, -50%);
                    width: 700px;
                    height: 400px;
                    border-radius: 50%;
                    border: 3px solid rgba(0, 255, 255, 0.5);
                    box-shadow: 0 0 30px rgba(0, 255, 255, 0.5);
                    opacity: 0;
                    animation: pulse 2s infinite;
                    pointer-events: none;
                    z-index: -1;
                }
                
                .pulse:nth-child(2) {
                    animation-delay: 0.5s;
                }
                
                .pulse:nth-child(3) {
                    animation-delay: 1s;
                }
                
                @keyframes pulse {
                    0% { 
                        transform: translate(-50%, -50%) scale(0.8);
                        opacity: 0.8;
                    }
                    100% { 
                        transform: translate(-50%, -50%) scale(1.3);
                        opacity: 0;
                    }
                }
            </style>
            <link href="https://fonts.googleapis.com/css2?family=Rajdhani:wght@500;700&display=swap" rel="stylesheet">
        </head>
        <body>
            <div class="neon-container">
                <div class="pulse"></div>
                <div class="pulse"></div>
                <div class="pulse"></div>
                
                <div class="heart-section">
                    <div class="neon-heart">♥</div>
                </div>
                
                <div class="rate-section">
                    <div class="neon-rate" id="heart-rate" data-rate="0">0</div>
                </div>
                
                <div class="status-dot" id="status-dot"></div>
                <div class="connection-status" id="connection-status">等待数据...</div>
                <div class="neon-border"></div>
                
                <div class="corner corner-tl"></div>
                <div class="corner corner-tr"></div>
                <div class="corner corner-bl"></div>
                <div class="corner corner-br"></div>
            </div>
            
            <script>
                let lastHeartRate = 0;
                let lastUpdate = Date.now();
                
                async function updateHeartRate() {
                    try {
                        const response = await fetch('/data');
                        if (!response.ok) return;
                        
                        const rateText = await response.text();
                        const rate = parseInt(rateText);
                        
                        if (!isNaN(rate)) {
                            document.getElementById('heart-rate').textContent = rate;
                            document.getElementById('heart-rate').setAttribute('data-rate', rate);
                            lastHeartRate = rate;
                            lastUpdate = Date.now();
                            
                            // 更新状态点颜色
                            document.getElementById('status-dot').style.background = '#00ff00';
                            document.getElementById('status-dot').style.boxShadow = '0 0 15px #00ff00';
                            document.getElementById('connection-status').textContent = '数据正常';
                        }
                    } catch (error) {
                        console.error('获取心率失败:', error);
                        document.getElementById('connection-status').textContent = '连接错误';
                    }
                    
                    // 如果超过5秒没有更新，显示警告
                    if (Date.now() - lastUpdate > 5000) {
                        document.getElementById('status-dot').style.background = '#ff0000';
                        document.getElementById('status-dot').style.boxShadow = '0 0 15px #ff0000';
                        document.getElementById('connection-status').textContent = '信号丢失';
                    }
                }
                
                // 动态调整心跳动画速度
                function adjustHeartbeat() {
                    const heart = document.querySelector('.neon-heart');
                    if (lastHeartRate > 0) {
                        // 根据心率计算动画时长（心率越高，跳动越快）
                        const duration = Math.max(400, 1200 - (lastHeartRate * 5));
                        heart.style.animationDuration = `${duration}ms`;
                    }
                }
                
                // 立即更新
                updateHeartRate();
                
                // 每秒更新一次
                setInterval(updateHeartRate, 1000);
                
                // 每200毫秒调整一次心跳速度
                setInterval(adjustHeartbeat, 200);
            </script>
        </body>
        </html>
        "#;
        
        let response = Response::from_string(html)
            .with_header(html_content_type.clone());
        request.respond(response).expect("响应请求失败");
    }
}

// 处理蓝牙设备
fn handle_device(discovered_device: AdvertisingDevice, heart_rate: Arc<Mutex<HeartRateMonitor>>) {
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
        let heart_rate_value = manufacturer_data.data[3];
        
        println!("{name} 心率: {heart_rate_value:?}");
        
        let mut monitor = heart_rate.lock().unwrap();
        monitor.update(heart_rate_value);
    }
}

// 启动蓝牙扫描
async fn start_bluetooth_scan(
    heart_rate: Arc<Mutex<HeartRateMonitor>>
) -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::default()
        .await
        .ok_or("蓝牙设备未找到...")?;
    adapter.wait_available().await?;

    println!("开始扫描小米设备...");
    println!("访问: http://localhost:1145 查看心率监测");
    
    let mut scan = adapter.scan(&[]).await?;

    while let Some(discovered_device) = scan.next().await {
        handle_device(discovered_device, heart_rate.clone());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let heart_rate = Arc::new(Mutex::new(HeartRateMonitor::new()));
    
    let hr_web = heart_rate.clone();
    thread::spawn(move || start_http_server(hr_web));

    start_bluetooth_scan(heart_rate).await?;
    
    Ok(())
}