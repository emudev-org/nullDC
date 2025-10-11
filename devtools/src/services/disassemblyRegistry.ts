/**
 * Global registry for disassembly views to enable cross-panel navigation
 */

import type { DisassemblyViewRef } from "../app/components/DisassemblyView";

interface RegisteredView {
  ref: DisassemblyViewRef;
  target: "sh4" | "arm7" | "dsp";
  id: string;
}

class DisassemblyRegistry {
  private views = new Map<string, RegisteredView>();
  private focusedViewId: string | null = null;

  /**
   * Register a disassembly view
   */
  register(id: string, ref: DisassemblyViewRef, target: "sh4" | "arm7" | "dsp"): void {
    this.views.set(id, { ref, target, id });
  }

  /**
   * Unregister a disassembly view
   */
  unregister(id: string): void {
    this.views.delete(id);
    if (this.focusedViewId === id) {
      this.focusedViewId = null;
    }
  }

  /**
   * Mark a view as focused
   */
  setFocused(id: string): void {
    if (this.views.has(id)) {
      this.focusedViewId = id;
    }
  }

  /**
   * Clear focused view
   */
  clearFocused(): void {
    this.focusedViewId = null;
  }

  /**
   * Navigate to an address in the best available disassembly view for the target
   * - Prioritizes the currently focused view
   * - Falls back to the largest view
   * - Highlights the address and focuses the view
   */
  navigateToAddress(target: "sh4" | "arm7" | "dsp", address: number): boolean {
    const targetViews = Array.from(this.views.values()).filter((v) => v.target === target);

    if (targetViews.length === 0) {
      console.warn(`No disassembly views found for target: ${target}`);
      return false;
    }

    // Try focused view first
    const focusedView = targetViews.find((v) => v.id === this.focusedViewId);
    if (focusedView) {
      focusedView.ref.scrollToAddress(address, true);
      focusedView.ref.focus();
      return true;
    }

    // Find largest view
    let largestView = targetViews[0];
    let largestSize = 0;

    for (const view of targetViews) {
      const size = view.ref.getSize();
      const area = size.width * size.height;
      if (area > largestSize) {
        largestSize = area;
        largestView = view;
      }
    }

    largestView.ref.scrollToAddress(address, true);
    largestView.ref.focus();
    return true;
  }

  /**
   * Get all registered views for debugging
   */
  getRegisteredViews(): Array<{ id: string; target: string; size: { width: number; height: number } }> {
    return Array.from(this.views.values()).map((v) => ({
      id: v.id,
      target: v.target,
      size: v.ref.getSize(),
    }));
  }
}

// Singleton instance
export const disassemblyRegistry = new DisassemblyRegistry();
