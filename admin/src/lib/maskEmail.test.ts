import { describe, expect, it } from 'vitest';
import { maskEmail } from './maskEmail';

describe('maskEmail', () => {
	it('先頭の 1 文字とドメインだけ見せる', () => {
		expect(maskEmail('iizawa1138@gmail.com')).toBe('i***@gmail.com');
		expect(maskEmail('a@b')).toBe('a***@b');
	});
	it('ローカル部の長さを漏らさない', () => {
		expect(maskEmail('ab@example.com')).toBe(maskEmail('abcdefghijkl@example.com'));
	});
	it('サロゲートペアの文字も壊さない', () => {
		expect(maskEmail('𠮷野@example.com')).toBe('𠮷***@example.com');
	});
	it('メールの形でないものは全部伏せ、空はそのまま', () => {
		expect(maskEmail('not-an-email')).toBe('***');
		expect(maskEmail('@example.com')).toBe('***');
		expect(maskEmail('')).toBe('');
		expect(maskEmail(null)).toBe('');
		expect(maskEmail(undefined)).toBe('');
	});
});
