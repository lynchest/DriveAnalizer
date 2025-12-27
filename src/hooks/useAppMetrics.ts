import { useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useStore, AppMetrics } from '../store/useStore';

export function useAppMetrics() {
    const { setAppMetrics } = useStore();

    const fetchAppMetrics = useCallback(async () => {
        try {
            const metrics = await invoke<AppMetrics>('get_app_metrics');
            setAppMetrics(metrics);
        } catch (error) {
            console.error('Failed to fetch app metrics:', error);
        }
    }, [setAppMetrics]);

    useEffect(() => {
        fetchAppMetrics();
        const interval = setInterval(fetchAppMetrics, 2000);
        return () => clearInterval(interval);
    }, [fetchAppMetrics]);
}
