<script lang="ts">
	import { getModelIds, setModelIds } from '$lib/api/model.svelte';
	import { APIFetch } from '$lib/api';
	import type { ModelIdsResp } from '$lib/api/types';
	import { RefreshCw } from '@lucide/svelte';
	import { _ } from 'svelte-i18n';
	import Button from '$lib/ui/Button.svelte';

	const modelIds = $derived(getModelIds()?.ids);
	let refreshing = $state(false);
</script>

<div class="my-2 flex items-center justify-between border-b border-outline pb-2">
	<div class="text-sm text-text/80">{$_('setting.upstream_model_list')}</div>
	<Button
		class="flex items-center gap-2 px-3 py-2"
		disabled={refreshing}
		onclick={async () => {
			refreshing = true;
			try {
				const res = await APIFetch<ModelIdsResp>('model/ids', {});
				if (res) setModelIds(res);
			} finally {
				refreshing = false;
			}
		}}
	>
		<RefreshCw class={`h-4 w-4 ${refreshing ? 'animate-spin' : ''}`} />
		{$_('setting.refresh_model_ids')}
	</Button>
</div>

{#if modelIds == undefined}
	<div class="mb-4 flex items-center justify-center p-6 text-lg">{$_('common.loading')}</div>
{:else}
	<div class="grow space-y-2 overflow-y-auto">
		{#each modelIds as modelId (modelId)}
			<div class="rounded-lg border border-outline px-3 py-2 font-mono text-sm">{modelId}</div>
		{/each}
	</div>
{/if}
