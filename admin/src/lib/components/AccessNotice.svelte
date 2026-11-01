<script lang="ts">
	const ADMIN_URL = 'https://admin.sotalive.net/admin-console';
	/** Access は通ったが、役割の表にいない */
	export let forbidden = false;
</script>

<div class="min-h-screen flex items-center justify-center bg-gradient-to-br from-slate-900 via-slate-800 to-slate-900">
	<div class="w-full max-w-md">
		<div class="bg-slate-800/50 backdrop-blur-xl rounded-2xl shadow-2xl border border-slate-700/50 p-8 text-center">
			<div class="inline-flex items-center justify-center w-16 h-16 rounded-full bg-gradient-to-br from-emerald-400 to-cyan-500 mb-4">
				<svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
				</svg>
			</div>
			<h1 class="text-2xl font-bold text-white">Admin Console</h1>
			<p class="text-slate-400 mt-2">SOTA/POTA Reference Manager</p>

			{#if forbidden}
				<p class="text-slate-300 mt-6 text-sm leading-relaxed">
					ログインはできましたが、このメールアドレスには、管理画面の役割が登録されていません。<br />
					登録したものと別のメールや Google アカウントで入っていないか、確かめてください。
				</p>

				<a
					href="/cdn-cgi/access/logout"
					class="mt-6 inline-block w-full py-3 px-4 bg-gradient-to-r from-emerald-500 to-cyan-500 hover:from-emerald-600 hover:to-cyan-600 text-white font-semibold rounded-lg shadow-lg shadow-emerald-500/25 transition-all duration-200"
				>
					ログアウトして、別のアカウントで入る
				</a>
			{:else}
				<p class="text-slate-300 mt-6 text-sm leading-relaxed">
					管理画面は、専用のアドレスから入ります。<br />
					メールの番号か Google でログインします。
				</p>

				<a
					href={ADMIN_URL}
					class="mt-6 inline-block w-full py-3 px-4 bg-gradient-to-r from-emerald-500 to-cyan-500 hover:from-emerald-600 hover:to-cyan-600 text-white font-semibold rounded-lg shadow-lg shadow-emerald-500/25 transition-all duration-200"
				>
					admin.sotalive.net で入る
				</a>
			{/if}
		</div>
	</div>
</div>
