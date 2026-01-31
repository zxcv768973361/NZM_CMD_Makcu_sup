// src/main.rs
use minke_driver::InputDevice;
use minke_driver::human::HumanDriver;
use minke_driver::nav::{NavEngine, NavResult};
use minke_driver::tower_defense::TowerDefenseApp;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    println!("========================================");
    println!("🚀 MINKE 智能控制中心");
    println!("========================================");

    // 1. 硬件驱动初始化
    let port_name = "COM9"; 
    let (sw, sh) = (1920, 1080);
    
    let driver_arc = match InputDevice::new(port_name, 115200, sw, sh) {
        Ok(d) => Arc::new(Mutex::new(d)),
        Err(e) => {
            panic!("❌ 错误: 硬件未连接 ({})", e);
        }
    };

    // 启动心跳线程
    let hb = Arc::clone(&driver_arc);
    thread::spawn(move || loop {
        if let Ok(mut d) = hb.lock() { d.heartbeat(); }
        thread::sleep(Duration::from_secs(1));
    });

    let human_driver = Arc::new(Mutex::new(
        HumanDriver::new(Arc::clone(&driver_arc), sw/2, sh/2)
    ));

    // 2. 初始化导航引擎
    let engine = Arc::new(NavEngine::new("ui_map.toml", Arc::clone(&human_driver)));
    println!("✅ 视觉引擎与 UI 地图已就绪");

    println!("👉 请在 3 秒内切换到游戏窗口...");
    thread::sleep(Duration::from_secs(3));

    // ==========================================
    // 🎯 任务主循环
    // ==========================================
    let target_objective = "空间站普通"; 

    loop {
        println!("\n🔄 [主控] 开始导航至目标: {}", target_objective);
        
        let result = engine.navigate(target_objective);

        match result {
            NavResult::Success => {
                println!("✅ [主控] 已到达目标界面");
                thread::sleep(Duration::from_secs(5));
            }
            
            NavResult::Handover(scene_id) => {
                println!("⚔️  [主控] 检测到控制权移交: [{}]", scene_id);

                if scene_id == "空间站普通" {
                    println!("🏗️  启动塔防地图策略逻辑...");
                    
                    let mut td_app = TowerDefenseApp::new(
                        Arc::clone(&human_driver),
                        Arc::clone(&engine) 
                    );
                    
                    // 运行塔防流程
                    td_app.run("terrain_01.json", "strategy_01.json");
                }
                
                println!("🏁 [主控] 塔防任务结束，回到 UI 导航模式");
                thread::sleep(Duration::from_secs(2));
            }
            
            NavResult::Failed => {
                println!("❌ [主控] 导航失败，重新扫描中...");
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}