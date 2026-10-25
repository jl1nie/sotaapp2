/**
 * メールアドレスを伏せ字にする。先頭の 1 文字とドメインだけ見せる(例: i***@gmail.com)。
 * 画面共有や肩越しの目から守るための表示で、サーバーから届く値そのものは変えない。
 */
export function maskEmail(email: string | null | undefined): string {
	if (!email) return '';
	const at = email.lastIndexOf('@');
	if (at <= 0) return '***';
	return `${[...email][0]}***${email.slice(at)}`;
}
