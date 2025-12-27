import { useState, useMemo } from 'react';
import { useStore, ProcessInfo } from '../store/useStore';
import { formatBytes } from '../utils/format';
import './TopProcesses.css';

// Get initials for process icon
function getInitials(name: string): string {
    return name.substring(0, 2).toUpperCase();
}

// Get rank badge class
function getRankClass(rank: number): string {
    if (rank === 1) return 'gold';
    if (rank === 2) return 'silver';
    if (rank === 3) return 'bronze';
    return 'default';
}

// Helper to generate consistent colors from strings
function stringToColor(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        hash = str.charCodeAt(i) + ((hash << 5) - hash);
    }
    const c = (hash & 0x00ffffff).toString(16).toUpperCase();
    return '#' + '00000'.substring(0, 6 - c.length) + c;
}

type SortKey = 'name' | 'read_bytes' | 'write_bytes' | 'total_bytes';
type SortOrder = 'asc' | 'desc';

function TopProcesses() {
    const { topProcesses, dataDisplayMode, processHistory } = useStore();
    const [sortKey, setSortKey] = useState<SortKey>('total_bytes');
    const [sortOrder, setSortOrder] = useState<SortOrder>('desc');

    const handleSort = (key: SortKey) => {
        if (sortKey === key) {
            setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
        } else {
            setSortKey(key);
            setSortOrder('desc');
        }
    };

    const sortedProcesses = useMemo(() => {
        let processesToDisplay = [...topProcesses];

        if (dataDisplayMode === 'alltime') {
            const mergedMap = new Map<string, ProcessInfo>();

            Object.entries(processHistory).forEach(([name, stats]) => {
                mergedMap.set(name, {
                    pid: 0,
                    name,
                    exe_path: null,
                    read_bytes: stats.read_bytes,
                    write_bytes: stats.write_bytes,
                    total_bytes: stats.read_bytes + stats.write_bytes
                });
            });

            topProcesses.forEach(p => {
                const existing = mergedMap.get(p.name);
                if (existing) {
                    existing.read_bytes += p.read_bytes;
                    existing.write_bytes += p.write_bytes;
                    existing.total_bytes += p.total_bytes;
                    if (p.exe_path) existing.exe_path = p.exe_path;
                } else {
                    mergedMap.set(p.name, { ...p });
                }
            });

            processesToDisplay = Array.from(mergedMap.values());
        }

        const sorted = processesToDisplay.sort((a, b) => {
            let comparison = 0;
            if (sortKey === 'name') {
                comparison = a.name.localeCompare(b.name);
            } else {
                comparison = a[sortKey] - b[sortKey];
            }
            return sortOrder === 'desc' ? -comparison : comparison;
        });

        return sorted.map((p, index) => ({
            ...p,
            rank: index + 1
        }));
    }, [topProcesses, processHistory, dataDisplayMode, sortKey, sortOrder]);

    const getSortIndicator = (key: SortKey) => {
        if (sortKey !== key) return <span className="sort-indicator">↕️</span>;
        return sortOrder === 'desc' ? 
            <span className="sort-indicator active">🔽</span> : 
            <span className="sort-indicator active">🔼</span>;
    };

    return (
        <div className="top-processes">
            <div className="top-processes__header">
                <div className="top-processes__title-row">
                    <h2>Top Processes</h2>
                    <span className={`data-mode-badge ${dataDisplayMode}`}>
                        {dataDisplayMode === 'session' ? '🕐 Oturum' : '📊 Tüm Zamanlar'}
                    </span>
                </div>
                <p>Disk I/O kullanımına göre en aktif 50 uygulama</p>
            </div>

            {sortedProcesses.length === 0 ? (
                <div className="empty-state">
                    <div className="empty-state__icon">📊</div>
                    <h3>Veri Bekleniyor</h3>
                    <p>Process I/O verileri toplanıyor, lütfen bekleyin...</p>
                </div>
            ) : (
                <div className="table-container">
                    <table className="process-table">
                        <thead>
                            <tr>
                                <th>#</th>
                                <th onClick={() => handleSort('name')} className="sortable">
                                    Uygulama {getSortIndicator('name')}
                                </th>
                                <th onClick={() => handleSort('read_bytes')} className="sortable">
                                    Okunan {getSortIndicator('read_bytes')}
                                </th>
                                <th onClick={() => handleSort('write_bytes')} className="sortable">
                                    Yazılan {getSortIndicator('write_bytes')}
                                </th>
                                <th onClick={() => handleSort('total_bytes')} className="sortable">
                                    Toplam {getSortIndicator('total_bytes')}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {sortedProcesses.map((process) => (
                                <tr key={process.name}>
                                    <td>
                                        <span className={`rank-badge ${getRankClass(process.rank)}`}>
                                            {process.rank}
                                        </span>
                                    </td>
                                    <td>
                                        <div className="process-name-cell">
                                            <div className="process-icon" style={{ backgroundColor: stringToColor(process.name) }}>
                                                {getInitials(process.name)}
                                            </div>
                                            <div className="process-info">
                                                <span className="process-name" title={process.exe_path || process.name}>
                                                    {process.name}
                                                </span>
                                                {process.exe_path && (
                                                    <span className="process-path">{process.exe_path}</span>
                                                )}
                                            </div>
                                        </div>
                                    </td>
                                    <td>{formatBytes(process.read_bytes)}</td>
                                    <td>{formatBytes(process.write_bytes)}</td>
                                    <td className="highlight">{formatBytes(process.total_bytes)}</td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
        </div>
    );
}

export default TopProcesses;
