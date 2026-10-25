<script lang="ts">
	import { onMount } from 'svelte';
	import {
		listAdminUsers, putAdminUser, deleteAdminUser,
		type AdminUser, type AdminUserRole
	} from '$lib/api';

	import { maskEmail } from '$lib/maskEmail';

	export let selfEmail = '';

	// 伏せ字で表示し、行ごとに「表示」で全体を見る(画面共有や肩越しの目から守る)
	let revealed: Record<string, boolean> = {};
	function toggle(email: string) {
		revealed = { ...revealed, [email]: !revealed[email] };
	}
	function label(email: string): string {
		return revealed[email] ? email : maskEmail(email);
	}

	let users: AdminUser[] = [];
	let loading = false;
	let message = '';
	let messageOk = true;

	let newEmail = '';
	let newRole: AdminUserRole = 'reviewer';

	async function load() {
		loading = true;
		const r = await listAdminUsers();
		if (r.success && r.users) {
			users = r.users;
			message = '';
		} else {
			message = r.message;
			messageOk = false;
		}
		loading = false;
	}

	async function add(e: Event) {
		e.preventDefault();
		const email = newEmail.trim().toLowerCase();
		if (!email) return;
		const r = await putAdminUser(email, newRole);
		message = r.message;
		messageOk = r.success;
		if (r.success) {
			newEmail = '';
			await load();
		}
	}

	async function changeRole(u: AdminUser, role: AdminUserRole) {
		if (u.role === role) return;
		const r = await putAdminUser(u.email, role);
		message = r.message;
		messageOk = r.success;
		await load();
	}

	async function remove(u: AdminUser) {
		if (!confirm(`${label(u.email)} を削除しますか?`)) return;
		const r = await deleteAdminUser(u.email);
		message = r.message;
		messageOk = r.success;
		if (r.success) await load();
	}

	onMount(load);
</script>

<div class="mt-12">
	<div class="mb-6">
		<h2 class="text-2xl font-bold text-white">管理者</h2>
		<p class="text-slate-400 mt-1">admin は全部、reviewer は山頂の地図の見直しだけ</p>
	</div>

	{#if message}
		<div class="mb-4 px-4 py-3 rounded-lg text-sm {messageOk ? 'bg-emerald-500/10 border border-emerald-500/30 text-emerald-400' : 'bg-red-500/10 border border-red-500/30 text-red-400'}">
			{message}
		</div>
	{/if}

	<div class="bg-slate-800/50 rounded-2xl border border-slate-700/50 overflow-hidden">
		{#if loading && users.length === 0}
			<div class="p-6 text-center text-slate-400">読み込み中...</div>
		{:else}
			<table class="w-full text-sm">
				<thead class="bg-slate-700/50">
					<tr>
						<th class="px-4 py-3 text-left text-slate-300 font-medium">メール</th>
						<th class="px-4 py-3 text-left text-slate-300 font-medium">役割</th>
						<th class="px-4 py-3 text-center text-slate-300 font-medium">操作</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-slate-700/30">
					{#each users as u (u.email)}
						<tr>
							<td class="px-4 py-3 text-slate-200">
								<span>{label(u.email)}</span>{u.email === selfEmail ? '(自分)' : ''}
								<button
									type="button"
									on:click={() => toggle(u.email)}
									class="ml-2 text-xs text-slate-400 hover:text-slate-200 underline"
								>{revealed[u.email] ? '隠す' : '表示'}</button>
							</td>
							<td class="px-4 py-3">
								<select
									value={u.role}
									on:change={(e) => changeRole(u, e.currentTarget.value as AdminUserRole)}
									class="px-2 py-1 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white"
								>
									<option value="admin">admin</option>
									<option value="reviewer">reviewer</option>
								</select>
							</td>
							<td class="px-4 py-3 text-center">
								<button
									on:click={() => remove(u)}
									disabled={u.email === selfEmail}
									class="px-3 py-1 text-xs font-medium text-red-400 bg-red-500/10 hover:bg-red-500/20 disabled:opacity-30 disabled:cursor-not-allowed rounded-lg transition-colors"
								>削除</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
		<form on:submit={add} class="flex flex-wrap gap-3 p-4 border-t border-slate-700/50">
			<input
				type="email"
				bind:value={newEmail}
				placeholder="メール"
				required
				class="px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white placeholder-slate-500 focus:outline-none focus:border-emerald-500 w-64"
			/>
			<select bind:value={newRole} class="px-3 py-2 bg-slate-800 border border-slate-600 rounded-lg text-sm text-white">
				<option value="reviewer">reviewer</option>
				<option value="admin">admin</option>
			</select>
			<button type="submit" class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 rounded-lg text-sm font-medium transition-colors text-white">追加</button>
		</form>
	</div>
</div>
