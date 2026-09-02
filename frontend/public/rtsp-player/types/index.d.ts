// ==================== 常量 ====================

export const Events: {
    readonly WINDOW_CREATED: "window_created";
    readonly WINDOW_DESTROYED: "window_destroyed";
    readonly WINDOW_RESIZED: "window_resized";
    readonly PLAYER_ADD: "player_add";
    readonly PLAYER_REMOVE: "player_remove";
    readonly PLAYER_LAYOUT_CHANGED: "player_layout_changed";
    readonly PLAYER_START: "player_start";
    readonly PLAYER_STOP: "player_stop";
    readonly PLAYER_CREATED: "player_created";
    readonly PLAYER_REMOVED: "player_removed";
    readonly FRAME_STATS: "frame_stats";
    readonly WS_STATUS: "ws_status";
    readonly PLAYER_STATE_CHANGED: "player_state_changed";
    readonly SERVER_NOTIFICATION: "server_notification";
    readonly ERROR: "error";
};

export const PlayerState: {
    readonly IDLE: "idle";
    readonly CONNECTING: "connecting";
    readonly PLAYING: "playing";
    readonly STOPPED: "stopped";
    readonly ERROR: "error";
};

export type PlayerStateType = "idle" | "connecting" | "playing" | "stopped" | "error";
export type UnsubscribeFn = () => void;

// ==================== 基础类型 ====================

export interface Region { left: number; top: number; width: number; height: number; }
export interface PhysicalRegion { x: number; y: number; width: number; height: number; }
export interface FrameStats { fps: number; width: number; height: number; pts: number; }
export interface WsStatusData { status: "connected" | "connecting" | "disconnected" | "error"; message?: string; }

// ==================== 配置类型 ====================

export interface WindowLayoutConfig { rows?: number; cols?: number; width?: number; height?: number; }

export interface WindowConfig { windowId?: string; title?: string; layout: WindowLayoutConfig; players?: PlayerConfig[]; }

/** 播放器配置（唯一一份） */
export interface PlayerConfig {
    rtspUrl: string;
    autoStart?: boolean;
    timeout?: number;
    layoutItemId?: string;
    rowSpan?: number;
    colSpan?: number;
    preferRow?: number;
    preferCol?: number;
    userData?: Record<string, any>;
}

export interface AddLayoutItemConfig {
    rowSpan?: number;
    colSpan?: number;
    preferRow?: number;
    preferCol?: number;
}

// ==================== 回调类型 ====================

export type MenuEnabledFn = (
    pw: PlayerWindow,
    layoutItem: LayoutItem,
    player: Player | null,
) => boolean;

export type MenuClickFn = (
    pw: PlayerWindow,
    layoutItem: LayoutItem,
    player: Player | null,
) => void;

export type EventCallback<T = any> = (
    data: T,
    playerWindow: PlayerWindow,
    player: Player | null,
) => void;

// ==================== 菜单/工具栏配置 ====================

export interface ContextMenuItemConfig {
    id: string;
    label: string;
    icon?: string;
    separator?: boolean;
    enabled?: MenuEnabledFn;
    onClick?: MenuClickFn;
}

export interface ToolbarButtonConfig {
    id: string;
    icon?: string;
    tooltip?: string;
    className?: string;
    enabled?: MenuEnabledFn;
    onClick?: MenuClickFn;
    position?: 'start' | 'end' | number;
    _builtin?: boolean;
}

// ==================== Player ====================

export class Player {
    readonly id: string;
    readonly windowId: string;
    readonly layoutItemId: string;
    rtspUrl: string;
    state: PlayerStateType;
    readonly createdAt: number;
    stats: FrameStats;
    region: Region;
    userData: Record<string, any>;
    constructor(playerId: string, windowId: string, layoutItemId: string, rtspUrl: string, userData?: Record<string, any>);
    getUserData(): Record<string, any>;
    updateUserData(data: Record<string, any>): void;
}

// ==================== LayoutItem ====================

export class LayoutItem {
    readonly id: string;
    readonly row: number;
    readonly col: number;
    readonly rowSpan: number;
    readonly colSpan: number;
    _occupied: boolean;
    get occupied(): boolean;
    constructor(id: string, row: number, col: number, rowSpan?: number, colSpan?: number);
}

// ==================== PlayerWindowLayout ====================

export class PlayerWindowLayout {
    get rows(): number;
    get cols(): number;
    get items(): LayoutItem[];
    constructor(rows?: number, cols?: number);
    setGrid(rows: number, cols: number): string[];
    allocate(id: string, rowSpan?: number, colSpan?: number, preferRow?: number, preferCol?: number): LayoutItem | null;
    release(id: string): void;
    get(id: string): LayoutItem | null;
    calcRegion(id: string, windowWidth: number, windowHeight: number, padding?: number): Region | null;
    calcAllRegions(windowWidth: number, windowHeight: number, padding?: number): Map<string, Region>;
    hasSpace(rowSpan?: number, colSpan?: number): boolean;
    destroy(): void;
}

// ==================== PlayerWindow ====================

export class PlayerWindow {
    readonly id: string;
    readonly isDestroyed: boolean;
    readonly element: HTMLDivElement | null;
    readonly canvas: HTMLCanvasElement | null;
    readonly players: Map<string, Player>;
    width: number;
    height: number;
    layout: PlayerWindowLayout;
    onClose: ((windowId: string) => void) | null;
    onPlayerStateChange: ((playerId: string, state: string, data?: any) => void) | null;
    onPlayerAdded: ((player: Player) => void) | null;
    onPlayerRemoved: ((playerId: string) => void) | null;
    onError: ((errorType: string, data: any) => void) | null;

    constructor(windowId: string, messageBus: MessageBus);

    on<T = any>(eventName: string, callback: EventCallback<T>): UnsubscribeFn;

    selectLayoutItem(layoutItemId: string | null): void;
    getSelectedLayoutItemId(): string | null;
    getSelectedLayoutItem(): LayoutItem | null;
    getSelectedPlayer(): Player | null;
    playOnSelected(rtspUrl: string, userData?: Record<string, any>): Promise<Player>;

    create(title: string, layoutConfig: WindowLayoutConfig, parentElement: HTMLElement): Promise<PlayerWindow>;
    getPlayerCount(): number;
    getLayoutInfo(): { rows: number; cols: number; items: Array<{ id: string; row: number; col: number; rowSpan: number; colSpan: number; occupied: boolean }> };

    addPlayer(config: PlayerConfig): Promise<Player>;
    removePlayer(playerId: string): Promise<void>;
    startPlayer(playerId: string, rtspUrl?: string): Promise<void>;
    stopPlayer(playerId: string): Promise<void>;
    close(): Promise<void>;
    resize(width: number, height: number): Promise<void>;

    addLayoutItem(config?: AddLayoutItemConfig): LayoutItem;
    removeLayoutItem(layoutItemId: string): void;

    registerContextMenuItem(menuItem: ContextMenuItemConfig): void;
    unregisterContextMenuItem(itemId: string): void;

    registerToolbarButton(buttonConfig: ToolbarButtonConfig): void;
    unregisterToolbarButton(buttonId: string): void;
    getToolbarButtons(): ToolbarButtonConfig[];

    onFrameStats(playerId: string, data: FrameStats): void;
    onPlayerStateChanged(playerId: string, state: string, data?: any): void;
    onWsStatus(data: WsStatusData): void;
    onServerNotificationForPlayer(playerId: string, data: any): void;
}

// ==================== MessageBus ====================

export class MessageBus {
    timeout: number;
    constructor(messagePort: Worker | MessagePort, options?: { timeout?: number });
    request<T = any>(action: string, data?: any, timeoutOrTransferList?: number | Transferable[], transferList?: Transferable[]): Promise<T>;
    sendResponse(id: string, data: any, transferList?: Transferable[]): void;
    sendError(id: string, code: number, message: string): void;
    notify(event: string, data?: any, transferList?: Transferable[]): void;
    on(event: string, callback: (data: any) => void): UnsubscribeFn;
    once(event: string, callback: (data: any) => void): UnsubscribeFn;
    off(event: string): void;
    onRequest(handler: (action: string, data: any) => Promise<any> | any): void;
}

// ==================== ClientApp ====================

export class ClientApp {
    readonly serverUrl: string;
    readonly worker: Worker;
    constructor(serverUrl: string);
    createWindow(config: WindowConfig, parentElement: HTMLElement): Promise<PlayerWindow>;
    closeWindow(windowId: string): Promise<void>;
    getWindow(windowId: string): PlayerWindow | undefined;
    getAllWindows(): PlayerWindow[];
    getWindowCount(): number;
    on(event: string, callback: (data: any) => void): UnsubscribeFn;
    destroy(): Promise<void>;
}

export function escapeHtml(str: string): string;