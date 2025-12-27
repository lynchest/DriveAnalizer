import React, { useEffect, useLayoutEffect, useRef } from 'react';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import './Chart.css';

interface ChartProps {
    data: uPlot.AlignedData;
}

const Chart: React.FC<ChartProps> = ({ data }) => {
    const chartRef = useRef<HTMLDivElement>(null);
    const uPlotInst = useRef<uPlot | null>(null);

    // Initial Chart Creation
    useLayoutEffect(() => {
        if (!chartRef.current) return;

        const opts: uPlot.Options = {
            title: "Disk I/O Activity",
            id: "io-chart",
            class: "io-chart-instance",
            width: chartRef.current.clientWidth,
            height: 340,
            series: [
                {
                    // x-axis (timestamp)
                    label: "Time",
                    value: (_self, rawValue) => rawValue ? new Date(rawValue * 1000).toLocaleTimeString() : '--',
                },
                {
                    // Read Speed
                    label: "Read",
                    stroke: "#4cc9f0",
                    width: 2,
                    fill: "rgba(76, 201, 240, 0.1)",
                    value: (_self, rawValue) => (rawValue != null ? formatBytes(rawValue) + "/s" : '--'),
                },
                {
                    // Write Speed
                    label: "Write",
                    stroke: "#f72585",
                    width: 2,
                    fill: "rgba(247, 37, 133, 0.1)",
                    value: (_self, rawValue) => (rawValue != null ? formatBytes(rawValue) + "/s" : '--'),
                }
            ],
            axes: [
                {
                    stroke: "#a0a0a0",
                    grid: { show: true, stroke: "#333", width: 1 },
                    ticks: { show: true, stroke: "#333", width: 1 },
                },
                {
                    stroke: "#a0a0a0",
                    grid: { show: true, stroke: "#333", width: 1 },
                    ticks: { show: true, stroke: "#333", width: 1 },
                    values: (_self, ticks) => ticks.map(t => formatBytes(t)),
                }
            ],
            scales: {
                x: {
                    time: true,
                }
            },
            cursor: {
                drag: { x: true, y: true },
            },
            legend: {
                show: true,
            }
        };

        const u = new uPlot(opts, data, chartRef.current);
        uPlotInst.current = u;

        // Resize Observer
        const observer = new ResizeObserver(entries => {
            if (!uPlotInst.current) return;
            const entry = entries[0];
            // Use border box to ensure it fits perfectly inside padding
            const width = entry.target.getBoundingClientRect().width;
            uPlotInst.current.setSize({ width, height: 340 });
        });

        observer.observe(chartRef.current);

        return () => {
            u.destroy();
            observer.disconnect();
            uPlotInst.current = null;
        };
    }, []); // Empty dependency array: Only destroy/create on mount/unmount

    // Efficient Data Update
    useEffect(() => {
        if (uPlotInst.current) {
            uPlotInst.current.setData(data);
        }
    }, [data]);

    return (
        <div className="chart-container">
            <div ref={chartRef} className="chart-wrapper" />
        </div>
    );
};

// Helper for formatting bytes
function formatBytes(bytes: number, decimals = 2) {
    if (!+bytes) return '0 B';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

export default Chart;
