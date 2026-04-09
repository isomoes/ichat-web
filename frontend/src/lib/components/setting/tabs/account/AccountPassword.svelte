<script lang="ts">
	import { _ } from 'svelte-i18n';
	import { getCurrentUser } from '$lib/api/user.svelte';
	import CheckPwd from '../../CheckPwd.svelte';
	import { updateUser } from '$lib/api/user.svelte';
	import { logout } from '$lib/api/auth';
	import Warning from '../../Warning.svelte';

	let { mutate, isError } = updateUser();
	const currentUser = $derived(getCurrentUser());
</script>

{#if isError()}
	<Warning>{$_('setting.account.error_updating_password')}</Warning>
{/if}
{#if currentUser?.external_auth}
	<p class="rounded-md border border-outline p-4 text-sm">
		Password changes are managed by your external account provider.
	</p>
{:else}
	<CheckPwd
		message={$_('setting.account.enter_new_password')}
		onsubmit={(password) => {
			mutate({ password }, () => {
				logout();
			});
		}}
	></CheckPwd>
{/if}
