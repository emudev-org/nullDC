/**
 * Service for navigating disassembly panels programmatically
 */

export interface DisassemblyPanelRef {
  scrollToAddress: (address: number, highlight?: boolean) => void;
  focus: () => void;
  getSize: () => { width: number; height: number };
  getTarget: () => string;
}

class DisassemblyNavigationService {
  private panels = new Map<string, DisassemblyPanelRef>();

  register(id: string, ref: DisassemblyPanelRef) {
    this.panels.set(id, ref);
  }

  unregister(id: string) {
    this.panels.delete(id);
  }

  navigateToAddress(target: "sh4" | "arm7" | "dsp", address: number) {
    // Find all panels for this target
    const targetPanels: Array<{ id: string; ref: DisassemblyPanelRef; size: number }> = [];

    for (const [id, ref] of this.panels.entries()) {
      if (ref.getTarget() === target) {
        const size = ref.getSize();
        targetPanels.push({ id, ref, size: size.width * size.height });
      }
    }

    if (targetPanels.length === 0) {
      console.warn(`No ${target} disassembly panel found`);
      return;
    }

    // Sort by size (biggest first)
    targetPanels.sort((a, b) => b.size - a.size);

    // Use the biggest panel
    const panel = targetPanels[0];
    panel.ref.scrollToAddress(address, true);
    panel.ref.focus();
  }

  getPanelsForTarget(target: string): DisassemblyPanelRef[] {
    const result: DisassemblyPanelRef[] = [];
    for (const ref of this.panels.values()) {
      if (ref.getTarget() === target) {
        result.push(ref);
      }
    }
    return result;
  }
}

export const disassemblyNavigationService = new DisassemblyNavigationService();
