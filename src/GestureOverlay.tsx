import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

interface GestureEvent {
  gesture: string;
  confidence: number;
  detected: boolean;
}

export function GestureOverlay() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [detection, setDetection] = useState<GestureEvent | null>(null);
  const [frameUrl, setFrameUrl] = useState<string | null>(null);

  useEffect(() => {
    const unlisten = listen<string>("camera-frame", (e) => setFrameUrl(e.payload));
    return () => { unlisten.then((f) => f()); };
  }, []);

  useEffect(() => {
    const unlisten = listen<GestureEvent>("gesture-detection", (e) =>
      setDetection(e.payload)
    );
    return () => { unlisten.then((f) => f()); };
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, canvas.width, canvas.height);

    if (!detection?.detected) return;

    const W = canvas.width;
    const H = canvas.height;
    const pct = Math.round(detection.confidence * 100);

    // Frame highlight
    ctx.strokeStyle = "#00ff88";
    ctx.lineWidth = 3;
    ctx.strokeRect(4, 4, W - 8, H - 8);

    // Label background
    const label = `${detection.gesture}  ${pct}%`;
    ctx.font = "bold 13px sans-serif";
    const textW = ctx.measureText(label).width;
    ctx.fillStyle = "rgba(0, 0, 0, 0.6)";
    ctx.fillRect(4, H - 32, textW + 16, 28);

    // Label text
    ctx.fillStyle = "#00ff88";
    ctx.fillText(label, 12, H - 12);

    // Confidence bar along bottom
    ctx.fillStyle = "rgba(0,255,136,0.25)";
    ctx.fillRect(4, H - 6, W - 8, 4);
    ctx.fillStyle = "#00ff88";
    ctx.fillRect(4, H - 6, (W - 8) * detection.confidence, 4);
  }, [detection]);

  return (
    <div
      style={{
        position: "fixed",
        top: 68,
        right: 16,
        width: 240,
        height: 180,
        borderRadius: 10,
        overflow: "hidden",
        border: "1px solid rgba(255,255,255,0.12)",
        backgroundColor: "#000",
        zIndex: 1000,
        boxShadow: "0 4px 24px rgba(0,0,0,0.5)",
      }}
    >
      {frameUrl
        ? <img src={frameUrl} style={{ width: "100%", height: "100%", objectFit: "cover", display: "block" }} />
        : <div style={{ position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center", color: "#666", fontSize: 11 }}>Waiting for camera…</div>
      }
      <canvas
        ref={canvasRef}
        width={240}
        height={180}
        style={{ position: "absolute", top: 0, left: 0, pointerEvents: "none" }}
      />
    </div>
  );
}
