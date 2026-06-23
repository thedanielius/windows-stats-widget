import { listen } from "@tauri-apps/api/event";

interface SystemStats {
  cpu: number;
  ram_pct: number;
  ram_used: number;
  ram_total: number;
  disk_read: number;
  disk_write: number;
  net_recv: number;
  net_sent: number;
  gpu: number;
}

class Sparkline {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private history: number[] = [];
  private maxPoints = 28;
  private lineColor: string;
  private fillColor: string;
  private isPercentage: boolean;
  private scaleMin: number;

  constructor(canvasId: string, lineColor: string, fillColor: string, isPercentage = false, scaleMin = 1) {
    const el = document.getElementById(canvasId);
    if (!el || !(el instanceof HTMLCanvasElement)) {
      throw new Error(`Canvas element #${canvasId} not found`);
    }
    this.canvas = el;
    const ctx = this.canvas.getContext("2d");
    if (!ctx) {
      throw new Error(`Failed to get 2D context for #${canvasId}`);
    }
    this.ctx = ctx;
    this.lineColor = lineColor;
    this.fillColor = fillColor;
    this.isPercentage = isPercentage;
    this.scaleMin = scaleMin;

    for (let i = 0; i < this.maxPoints; i++) {
      this.history.push(0);
    }

    this.resize();
    window.addEventListener("resize", () => this.resize());
  }

  private resize() {
    const dpr = window.devicePixelRatio || 1;
    const rect = this.canvas.getBoundingClientRect();
    this.canvas.width = rect.width * dpr;
    this.canvas.height = rect.height * dpr;
    this.ctx.resetTransform();
    this.ctx.scale(dpr, dpr);
    this.draw();
  }

  updateColors(lineColor: string, fillColor: string) {
    this.lineColor = lineColor;
    this.fillColor = fillColor;
    this.draw();
  }

  addPoint(value: number) {
    this.history.push(value);
    if (this.history.length > this.maxPoints) {
      this.history.shift();
    }
    this.draw();
  }

  private draw() {
    const dpr = window.devicePixelRatio || 1;
    const width = this.canvas.width / dpr;
    const height = this.canvas.height / dpr;

    this.ctx.clearRect(0, 0, width, height);

    let max = this.scaleMin;
    if (this.isPercentage) {
      max = 100;
    } else {
      max = Math.max(this.scaleMin, ...this.history);
    }

    const count = this.history.length;
    if (count < 2) return;

    const dx = width / (count - 1);

    this.ctx.beginPath();

    for (let i = 0; i < count; i++) {
      const x = i * dx;
      const y = height - (this.history[i] / max) * (height - 2) - 1;

      if (i === 0) {
        this.ctx.moveTo(x, y);
      } else {
        this.ctx.lineTo(x, y);
      }
    }

    this.ctx.strokeStyle = this.lineColor;
    this.ctx.lineWidth = 1.5;
    this.ctx.lineCap = "round";
    this.ctx.lineJoin = "round";
    this.ctx.stroke();

    this.ctx.lineTo((count - 1) * dx, height);
    this.ctx.lineTo(0, height);
    this.ctx.closePath();

    const grad = this.ctx.createLinearGradient(0, 0, 0, height);
    grad.addColorStop(0, this.fillColor);
    grad.addColorStop(1, "rgba(0, 0, 0, 0)");

    this.ctx.fillStyle = grad;
    this.ctx.fill();
  }
}

function formatSpeed(bytes: number): string {
  if (bytes < 1024) return `${Math.round(bytes)}B`;
  const kb = bytes / 1024;
  if (kb < 1000) return `${Math.round(kb)}K`;
  const mb = kb / 1024;
  if (mb < 1000) return `${Math.round(mb)}M`;
  return `${(mb / 1024).toFixed(1)}G`;
}

const COLORS = {
  cpu: { line: "#00d2ff", fill: "rgba(0, 210, 255, 0.22)" },
  ram: { line: "#00f5d4", fill: "rgba(0, 245, 212, 0.22)" },
  disk: { line: "#ff9f1c", fill: "rgba(255, 159, 28, 0.22)" },
  net: { line: "#4ade80", fill: "rgba(74, 222, 128, 0.22)" },
  gpu: { line: "#d946ef", fill: "rgba(217, 70, 239, 0.22)" },
};

window.addEventListener("DOMContentLoaded", async () => {
  const cpuSparkline = new Sparkline("graph-cpu", COLORS.cpu.line, COLORS.cpu.fill, true);
  const ramSparkline = new Sparkline("graph-ram", COLORS.ram.line, COLORS.ram.fill, true);
  const diskSparkline = new Sparkline("graph-disk", COLORS.disk.line, COLORS.disk.fill, false, 1024 * 1024);
  const netSparkline = new Sparkline("graph-net", COLORS.net.line, COLORS.net.fill, false, 100 * 1024);
  const gpuSparkline = new Sparkline("graph-gpu", COLORS.gpu.line, COLORS.gpu.fill, true);

  const cpuVal = document.getElementById("val-cpu")!;
  const ramVal = document.getElementById("val-ram")!;
  const diskVal = document.getElementById("val-disk")!;
  const netVal = document.getElementById("val-net")!;
  const gpuVal = document.getElementById("val-gpu")!;

  await listen<SystemStats>("stats-update", (event) => {
    const stats = event.payload;

    cpuVal.textContent = `${Math.round(stats.cpu)}%`;
    cpuSparkline.addPoint(stats.cpu);

    ramVal.textContent = `${Math.round(stats.ram_pct)}%`;
    ramSparkline.addPoint(stats.ram_pct);

    const diskTotal = stats.disk_read + stats.disk_write;
    diskVal.textContent = formatSpeed(diskTotal);
    diskSparkline.addPoint(diskTotal);

    const netTotal = stats.net_recv + stats.net_sent;
    netVal.textContent = formatSpeed(netTotal);
    netSparkline.addPoint(netTotal);

    gpuVal.textContent = `${Math.round(stats.gpu)}%`;
    gpuSparkline.addPoint(stats.gpu);
  });
});
