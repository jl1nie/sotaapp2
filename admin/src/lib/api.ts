import { auth } from './auth';

export interface UploadResult {
	success: boolean;
	message: string;
	imported?: number;
	skipped?: number;
	code?: string;
}

interface ApiResponse {
	success: boolean;
	message: string;
	imported?: number;
	skipped?: number;
	code?: string;
	errors?: Array<{ line: number; message: string }>;
}

async function uploadFile(endpoint: string, file: File): Promise<UploadResult> {
	const token = auth.getToken();
	if (!token) {
		return { success: false, message: '認証されていません' };
	}

	const formData = new FormData();
	formData.append('file', file);

	try {
		const response = await fetch(endpoint, {
			method: 'POST',
			headers: { 'Authorization': `Bearer ${token}` },
			body: formData
		});

		if (response.status === 401 || response.status === 403) {
			auth.logout();
			return { success: false, message: 'セッションが期限切れです。再度ログインしてください。' };
		}

		// Try to parse JSON response
		let data: ApiResponse;
		try {
			data = await response.json();
		} catch {
			// If JSON parsing fails, return generic error
			if (!response.ok) {
				return { success: false, message: `アップロードに失敗しました: ${response.status} ${response.statusText}` };
			}
			return { success: true, message: 'アップロード成功！' };
		}

		// Return parsed response
		if (data.success) {
			return {
				success: true,
				message: data.message || `インポート完了: ${data.imported ?? 0}件追加`,
				imported: data.imported,
				skipped: data.skipped
			};
		} else {
			return {
				success: false,
				message: data.message || 'アップロードに失敗しました',
				code: data.code
			};
		}
	} catch (e) {
		return { success: false, message: 'ネットワークエラー。再試行してください。' };
	}
}

export async function uploadPotaParks(file: File): Promise<UploadResult> {
	return uploadFile('/api/v2/pota/import', file);
}

export async function uploadSotaJaSummits(file: File): Promise<UploadResult> {
	return uploadFile('/api/v2/sota/import/ja', file);
}

export async function uploadJccJcg(file: File): Promise<UploadResult> {
	return uploadFile('/api/v2/locator/jcc-jcg/import', file);
}

// Award judgment types
export type JudgmentMode = 'strict' | 'lenient';
export type LogType = 'unknown' | 'activator' | 'chaser';

export interface SummitActivation {
	summitCode: string;
	uniqueStations: number;
	qualified: boolean;
}

export interface ActivatorAwardResult {
	achieved: boolean;
	qualifiedSummits: number;
	summits: SummitActivation[];
}

export interface SummitChase {
	summitCode: string;
	uniqueActivators: number;
	activators: string[];
}

export interface ChaserAwardResult {
	achieved: boolean;
	qualifiedSummits: SummitChase[];
}

export interface AwardJudgmentResult {
	success: boolean;
	callsign: string;
	totalQsos: number;
	logType: LogType;
	activator?: ActivatorAwardResult;
	chaser?: ChaserAwardResult;
	mode: JudgmentMode;
}

export interface AwardJudgmentResponse {
	success: boolean;
	result?: AwardJudgmentResult;
	message?: string;
}

// System metrics types
export interface SystemMetrics {
	uptime_secs: number;
	memory_used_bytes: number | null;
	memory_used_mb: number | null;
	db_status: string;
}

export interface MetricsResponse {
	success: boolean;
	data?: SystemMetrics;
	message?: string;
}

export interface RestartResponse {
	success: boolean;
	message: string;
}

export async function getSystemMetrics(): Promise<MetricsResponse> {
	const token = auth.getToken();
	if (!token) {
		return { success: false, message: '認証されていません' };
	}

	try {
		const response = await fetch('/api/v2/admin/metrics', {
			method: 'GET',
			headers: { 'Authorization': `Bearer ${token}` }
		});

		if (response.status === 401 || response.status === 403) {
			auth.logout();
			return { success: false, message: 'セッションが期限切れです。再度ログインしてください。' };
		}

		if (!response.ok) {
			return { success: false, message: `メトリクス取得に失敗しました: ${response.status}` };
		}

		const data: SystemMetrics = await response.json();
		return { success: true, data };
	} catch (e) {
		return { success: false, message: 'ネットワークエラー。再試行してください。' };
	}
}

export async function restartServer(): Promise<RestartResponse> {
	const token = auth.getToken();
	if (!token) {
		return { success: false, message: '認証されていません' };
	}

	try {
		const response = await fetch('/api/v2/admin/restart', {
			method: 'POST',
			headers: { 'Authorization': `Bearer ${token}` }
		});

		if (response.status === 401 || response.status === 403) {
			auth.logout();
			return { success: false, message: 'セッションが期限切れです。再度ログインしてください。' };
		}

		if (!response.ok) {
			return { success: false, message: `リスタートに失敗しました: ${response.status}` };
		}

		return { success: true, message: 'サーバーをリスタートしています...' };
	} catch (e) {
		return { success: false, message: 'ネットワークエラー。再試行してください。' };
	}
}

export async function judgeAward(file: File, mode: JudgmentMode = 'strict'): Promise<AwardJudgmentResponse> {
	const formData = new FormData();
	formData.append('file', file);

	try {
		const response = await fetch(`/api/v2/sota/award/10th-anniversary/judge?mode=${mode}`, {
			method: 'POST',
			body: formData
		});

		if (!response.ok) {
			const errorData = await response.json().catch(() => ({}));
			return {
				success: false,
				message: errorData.message || `判定に失敗しました: ${response.status} ${response.statusText}`
			};
		}

		const data: AwardJudgmentResult = await response.json();
		return {
			success: true,
			result: data
		};
	} catch (e) {
		return {
			success: false,
			message: 'ネットワークエラー。再試行してください。'
		};
	}
}

