import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface DiskStat {
    timestamp: number;
    read_speed: number;  // bytes/sec
    write_speed: number; // bytes/sec
    read_bytes: number;  // total bytes read
    write_bytes: number; // total bytes written
    idle_time: number;   // %
    queue_depth: number; // count
}

export interface ProcessInfo {
    pid: number;
    name: string;
    exe_path: string | null;
    read_bytes: number;
    write_bytes: number;
    total_bytes: number;
}

export type DataDisplayMode = 'session' | 'alltime';

export interface AllTimeTotals {
    read_bytes: number;
    write_bytes: number;
}

export interface AppMetrics {
    total_disk_size: number;
    ram_usage: number;
    cpu_usage: number;
}

interface AppState {
    currentStats: DiskStat;
    history: [number[], number[], number[]];
    topProcesses: ProcessInfo[];
    activePage: 'dashboard' | 'processes' | 'settings';
    windowSize: { width: number, height: number };
    dataDisplayMode: DataDisplayMode;
    allTimeTotals: AllTimeTotals;
    processHistory: Record<string, { read_bytes: number, write_bytes: number }>;
    appMetrics: AppMetrics;
    
    // Actions
    updateStats: (stat: DiskStat) => void;
    setHistory: (history: [number[], number[], number[]]) => void;
    setTopProcesses: (processes: ProcessInfo[]) => void;
    setActivePage: (page: 'dashboard' | 'processes' | 'settings') => void;
    setWindowSize: (width: number, height: number) => void;
    setDataDisplayMode: (mode: DataDisplayMode) => void;
    setAllTimeTotals: (totals: AllTimeTotals) => void;
    setProcessHistory: (history: Record<string, { read_bytes: number, write_bytes: number }>) => void;
    setAppMetrics: (metrics: AppMetrics) => void;
    resetSessionData: () => void;
}

const MAX_HISTORY_POINTS = 3600;

export const useStore = create<AppState>()(
    persist(
        (set) => ({
            currentStats: {
                timestamp: 0,
                read_speed: 0,
                write_speed: 0,
                read_bytes: 0,
                write_bytes: 0,
                idle_time: 0,
                queue_depth: 0,
            },
            history: [[], [], []],
            topProcesses: [],
            activePage: 'dashboard',
            windowSize: { width: 1280, height: 800 },
            dataDisplayMode: 'alltime',
            allTimeTotals: { read_bytes: 0, write_bytes: 0 },
            processHistory: {},
            appMetrics: { total_disk_size: 0, ram_usage: 0, cpu_usage: 0 },

            updateStats: (stat: DiskStat) => set((state) => {
                const newTimestamps = [...state.history[0], stat.timestamp];
                const newReads = [...state.history[1], stat.read_speed];
                const newWrites = [...state.history[2], stat.write_speed];

                if (newTimestamps.length > MAX_HISTORY_POINTS) {
                    newTimestamps.shift();
                    newReads.shift();
                    newWrites.shift();
                }

                return {
                    currentStats: stat,
                    history: [newTimestamps, newReads, newWrites],
                };
            }),

            setHistory: (history: [number[], number[], number[]]) => set({ history }),
            setTopProcesses: (processes: ProcessInfo[]) => set({ topProcesses: processes }),
            setActivePage: (page: 'dashboard' | 'processes' | 'settings') => set({ activePage: page }),
            setWindowSize: (width: number, height: number) => set({ windowSize: { width, height } }),
            setDataDisplayMode: (mode: DataDisplayMode) => set({ dataDisplayMode: mode }),
            setAllTimeTotals: (totals: AllTimeTotals) => set({ allTimeTotals: totals }),
            setProcessHistory: (history: Record<string, { read_bytes: number, write_bytes: number }>) => set({ processHistory: history }),
            setAppMetrics: (metrics: AppMetrics) => set({ appMetrics: metrics }),
            resetSessionData: () => set({
                currentStats: {
                    timestamp: 0,
                    read_speed: 0,
                    write_speed: 0,
                    read_bytes: 0,
                    write_bytes: 0,
                    idle_time: 0,
                    queue_depth: 0,
                },
                history: [[], [], []],
                topProcesses: [],
                allTimeTotals: { read_bytes: 0, write_bytes: 0 },
            }),
        }),
        {
            name: 'drive-analizer-storage',
            partialize: (state: AppState) => ({ 
                windowSize: state.windowSize,
                activePage: state.activePage,
                dataDisplayMode: state.dataDisplayMode
            }),
        }
    )
);

