import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function uid() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

export function scoreClass(score: number) {
  if (score >= 78) return "high";
  if (score >= 64) return "medium";
  return "low";
}

export function pathBasename(p: string) {
  return p.split(/[\\/]/).pop() || p;
}

export function urlBasename(url: string) {
  try {
    return new URL(url).pathname.split("/").pop() || "image";
  } catch {
    return "image";
  }
}
