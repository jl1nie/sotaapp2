import { writable } from 'svelte/store';

interface AuthState {
	token: string | null;
	email: string | null;
	isAuthenticated: boolean;
	loading: boolean;
}

function createAuthStore() {
	const { subscribe, set, update } = writable<AuthState>({
		token: null,
		email: null,
		isAuthenticated: false,
		loading: true
	});

	return {
		subscribe,
		init: () => {
			const token = localStorage.getItem('authToken');
			const email = localStorage.getItem('authEmail');
			if (token) {
				set({ token, email, isAuthenticated: true, loading: false });
			} else {
				set({ token: null, email: null, isAuthenticated: false, loading: false });
			}
		},
		login: async (email: string, password: string): Promise<{ success: boolean; error?: string }> => {
			try {
				const response = await fetch('/api/v2/auth/signin', {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ email, password })
				});

				if (!response.ok) {
					return { success: false, error: 'Invalid email or password' };
				}

				const token = response.headers.get('Authorization');
				if (!token) {
					return { success: false, error: 'No authorization token received' };
				}

				localStorage.setItem('authToken', token);
				localStorage.setItem('authEmail', email);
				set({ token, email, isAuthenticated: true, loading: false });
				return { success: true };
			} catch (e) {
				return { success: false, error: 'Network error. Please try again.' };
			}
		},
		logout: () => {
			localStorage.removeItem('authToken');
			localStorage.removeItem('authEmail');
			set({ token: null, email: null, isAuthenticated: false, loading: false });
		},
		getToken: (): string | null => {
			return localStorage.getItem('authToken');
		}
	};
}

export const auth = createAuthStore();
