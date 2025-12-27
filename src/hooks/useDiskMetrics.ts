import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useStore, DiskStat } from '../store/useStore';

const isValidDiskStat = (payload: unknown): payload is DiskStat => {
    if (!payload || typeof payload !== 'object') return false;
    const p = payload as Record<string, unknown>;
    return (
        typeof p.timestamp === 'number' &&
        typeof p.read_speed === 'number' &&
        typeof p.write_speed === 'number' &&
        typeof p.read_bytes === 'number' &&
        typeof p.write_bytes === 'number' &&
        p.read_speed >= 0 &&
        p.write_speed >= 0
    );
};

export function useDiskMetrics() {
    const { updateStats } = useStore();
    const lastUpdateRef = useRef<number>(0);
    const THROTTLE_MS = 100;

    useEffect(() => {
        const unlistenPromise = listen<DiskStat>('disk-metrics', (event) => {
            const now = Date.now();

            if (now - lastUpdateRef.current < THROTTLE_MS) {
                return;
            }

            if (!isValidDiskStat(event.payload)) {
                console.warn('Invalid disk-metrics payload received');
                return;
            }

            lastUpdateRef.current = now;
            updateStats(event.payload);
        });

        return () => {
            unlistenPromise.then((unlisten) => unlisten());
        };
    }, [updateStats]);
}
