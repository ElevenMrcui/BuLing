import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "不令 · BuLing",
  description: "去中心化多智能体协作平台（企业级）",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="zh-CN">
      <body>
        <header className="topbar">
          <div className="brand">
            <div className="mark">令</div>
            <div>
              <b>不令</b>
              <span>其身正，不令而行</span>
            </div>
          </div>
          <nav>
            <a href="/tasks">任务</a>
            <a href="/agents">Agent 中心</a>
            <a href="/providers">模型接入</a>
          </nav>
        </header>
        <main>{children}</main>
      </body>
    </html>
  );
}
