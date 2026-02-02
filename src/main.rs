// src/main.rs
use minke_driver::InputDevice;
use minke_driver::human::HumanDriver;
use minke_driver::nav::{NavEngine, NavResult}; // 确保导入 NavResult
use minke_driver::tower_defense::TowerDefenseApp;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    println!("========================================");
    println!("🚀 MINKE 智能控制中心 - 自动导航 + 塔防布阵");
    println!("========================================");

    // 1. 硬件驱动初始化
    let port_name = "COM9"; 
    let (sw, sh) = (1920, 1080);
    
    let driver_arc = match InputDevice::new(port_name, 115200, sw, sh) {
        Ok(d) => Arc::new(Mutex::new(d)),
        Err(_e) => {
            // panic!("❌ 错误: 硬件未连接 ({})", e); 
            // 注意：transmute 是极度危险的操作，仅用于无硬件环境下的逻辑编译测试
            unsafe { std::mem::transmute(Arc::new(Mutex::new(()))) } 
        }
    };

    // 启动心跳线程，维持硬件连接
    let hb = Arc::clone(&driver_arc);
    thread::spawn(move || loop {
        if let Ok(mut d) = hb.lock() { d.heartbeat(); }
        thread::sleep(Duration::from_secs(1));
    });

    // 2. 初始化驱动与引擎
    let human_driver = Arc::new(Mutex::new(
        HumanDriver::new(Arc::clone(&driver_arc), sw/2, sh/2)
    ));

    // 加载 UI 导航地图
    let engine = Arc::new(NavEngine::new("ui_map.toml", Arc::clone(&human_driver)));
    println!("✅ 视觉引擎与 UI 路径地图已就绪");

    println!("👉 请在 5 秒内切换到游戏窗口...");
    thread::sleep(Duration::from_secs(5));

    // ==========================================
    // 🎯 目标定位与控制权移交
    // ==========================================
    let target_page = "空间站普通"; // 此 ID 必须与 ui_map.toml 中的 scene.id 一致

    println!("\n🔄 [主控] 正在导航至目标界面: {}...", target_page);

    // 调用导航功能
    let nav_result = engine.navigate(target_page);

    match nav_result {
        // 当识别到进入了虚拟场景（战斗关卡入口）时
        NavResult::Handover(scene_id) => {
            println!("⚔️ [主控] 检测到控制权移交: [{}]", scene_id);
            println!("🏗️ 启动塔防自动化布防逻辑...");

            let mut td_app = TowerDefenseApp::new(
                Arc::clone(&human_driver),
                Arc::clone(&engine) 
            );

            // 配置要携带的塔
            let my_loadout = vec![
                "破坏者", 
                "自修复磁暴塔", 
                "防空导弹",
                "修理站"
            ];

            // 启动全自动塔防循环（包含波次监控和布阵）
            td_app.run(
                "空间站.json", 
                "strategy_01.json", 
                "traps_config.json", 
                &my_loadout          
            );
        }

        NavResult::Success => {
            println!("✅ [主控] 已成功到达目标 UI 界面，但未触发战斗入口。");
        }

        NavResult::Failed => {
            println!("❌ [主控] 导航失败：未能找到前往 {} 的路径或识别超时。", target_page);
        }
    }

    println!("🏁 [主控] 任务进程结束。");
}