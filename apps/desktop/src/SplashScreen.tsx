import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ArrowRightLeft, Database, MessageSquareText } from "lucide-react";
import { useRef } from "react";

gsap.registerPlugin(useGSAP);

type SplashScreenProps = {
  durationMs?: number;
  onComplete: () => void;
};

export default function SplashScreen({ durationMs = 1800, onComplete }: SplashScreenProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const reducedMotion =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  useGSAP(
    () => {
      if (reducedMotion) {
        const timeoutId = window.setTimeout(onComplete, 0);
        return () => {
          window.clearTimeout(timeoutId);
        };
      }

      const timeScale = Math.max(durationMs / 1800, 0.001);
      const splashRoot = rootRef.current;
      const timeline = gsap.timeline({
        defaults: { ease: "power3.out" },
        onComplete
      });

      gsap.set(".splash-stage", { autoAlpha: 0, scale: 0.96, y: 16 });
      gsap.set(".splash-flow-node", { autoAlpha: 0, scale: 0.68, y: 18 });
      gsap.set(".splash-line", { autoAlpha: 0, scaleX: 0.2 });
      gsap.set(".splash-word", { autoAlpha: 0, y: 14 });
      gsap.set(".splash-ring", { autoAlpha: 0, scale: 0.72, rotation: -18 });
      gsap.set(".splash-status", { autoAlpha: 0, y: 8 });

      timeline
        .to(".splash-stage", { autoAlpha: 1, scale: 1, y: 0, duration: 0.22 * timeScale })
        .to(".splash-ring", { autoAlpha: 1, scale: 1, rotation: 0, duration: 0.32 * timeScale }, "<0.05")
        .to(
          ".splash-flow-node",
          {
            autoAlpha: 1,
            scale: 1,
            y: 0,
            stagger: { each: 0.06 * timeScale, from: "center" },
            duration: 0.24 * timeScale
          },
          "<0.04"
        )
        .to(".splash-line", { autoAlpha: 1, scaleX: 1, stagger: 0.04 * timeScale, duration: 0.22 * timeScale }, "<0.08")
        .to(".splash-word", { autoAlpha: 1, y: 0, stagger: 0.08 * timeScale, duration: 0.24 * timeScale }, "<0.08")
        .to(".splash-status", { autoAlpha: 1, y: 0, duration: 0.18 * timeScale }, "<0.12")
        .to(".splash-flow-node", { y: -5, stagger: 0.025 * timeScale, duration: 0.18 * timeScale }, ">-0.02")
        .to(".splash-flow-node", { y: 0, stagger: 0.025 * timeScale, duration: 0.16 * timeScale }, ">-0.08")
        .to(splashRoot, { autoAlpha: 0, scale: 1.01, duration: 0.26 * timeScale }, ">0.14");

      return () => {
        timeline.kill();
      };
    },
    { scope: rootRef }
  );

  return (
    <div
      aria-label="AI Session Migrator 启动闪屏"
      aria-live="polite"
      className="splash-screen"
      ref={rootRef}
      role="status"
    >
      <section className="splash-stage" aria-hidden="false">
        <div className="splash-flow" aria-label="provider 会话流">
          <span aria-label="会话流节点 来源 provider" className="splash-flow-node provider">
            funai
          </span>
          <span className="splash-line" aria-hidden="true" />
          <span aria-label="会话流节点 会话记录" className="splash-flow-node session">
            <MessageSquareText aria-hidden="true" size={18} />
          </span>
          <span className="splash-line" aria-hidden="true" />
          <span aria-label="会话流节点 迁移核心" className="splash-flow-node center">
            <ArrowRightLeft aria-hidden="true" size={24} />
          </span>
          <span className="splash-line" aria-hidden="true" />
          <span aria-label="会话流节点 本地数据" className="splash-flow-node session">
            <Database aria-hidden="true" size={18} />
          </span>
          <span className="splash-line" aria-hidden="true" />
          <span aria-label="会话流节点 目标 provider" className="splash-flow-node provider target">
            yihubangg
          </span>
        </div>

        <div className="splash-brand">
          <span className="splash-ring" aria-hidden="true">
            <ArrowRightLeft size={34} />
          </span>
          <div>
            <p className="splash-word splash-kicker">Codex 会话迁移助手</p>
            <h2 className="splash-word">AI Session Migrator</h2>
            <p className="splash-word splash-copy">整理会话、切换 provider、继续你的工作流。</p>
          </div>
        </div>

        <div className="splash-status">
          <span aria-hidden="true" />
          正在准备本地会话环境
        </div>
      </section>
    </div>
  );
}
