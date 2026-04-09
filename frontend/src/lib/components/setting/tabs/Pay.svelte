<script lang="ts">
	import {
		getPayAmount,
		getSelfQuota,
		startPay,
		useSelfQuotaQueryEffect
	} from '$lib/api/payment.svelte';
	import Warning from '$lib/components/setting/Warning.svelte';
	import Button from '$lib/ui/Button.svelte';
	import Input from '$lib/ui/Input.svelte';

	const quotaPerYuan = 500000;

	useSelfQuotaQueryEffect();

	const selfQuota = $derived(getSelfQuota()?.data);
	const remainQuota = $derived(selfQuota?.quota);
	const remainMoney = $derived(
		remainQuota === undefined ? undefined : Number((remainQuota / quotaPerYuan).toFixed(2))
	);

	let amountInput = $state<string | number>('');
	let quotedAmount = $state<number | string | undefined>(undefined);
	let quoteError = $state('');
	let payError = $state('');

	type PayRequest = {
		amount: number;
		payment_method: string;
	};

	type PayResponseLike = {
		data?: unknown;
		url?: unknown;
	};

	const { mutate: quoteAmount, isPending: isQuotePending, isError: isQuoteError } = getPayAmount();
	const payMutation = startPay();
	const pay: (param: PayRequest) => Promise<PayResponseLike | undefined> = payMutation.mutate;
	const isPayPending = payMutation.isPending;
	const isPayError = payMutation.isError;

	function parseAmount(value: string | number): number | undefined {
		if (typeof value === 'number') {
			if (!Number.isSafeInteger(value) || value <= 0) return undefined;
			return value;
		}

		if (!/^\d+$/.test(value.trim())) return undefined;

		const parsed = Number.parseInt(value, 10);
		if (!Number.isSafeInteger(parsed) || parsed <= 0) return undefined;
		return parsed;
	}

	function formatMoney(value: number) {
		return new Intl.NumberFormat('zh-CN', {
			minimumFractionDigits: 2,
			maximumFractionDigits: 2
		}).format(value);
	}

	function extractQuotedAmount(
		response: { data?: unknown } | undefined
	): number | string | undefined {
		const data = response?.data;

		if (typeof data === 'number' || typeof data === 'string') return data;

		if (typeof data !== 'object' || data === null) return undefined;

		const objectData = data as Record<string, unknown>;
		const candidate =
			objectData.amount ??
			objectData.actual_amount ??
			objectData.need_pay_amount ??
			objectData.pay_amount ??
			objectData.money;

		if (typeof candidate === 'number' || typeof candidate === 'string') return candidate;
	}

	function extractPayUrl(response: { data?: unknown } | undefined): string | undefined {
		if (typeof response === 'object' && response !== null && 'url' in response) {
			const topLevelUrl = response.url;
			if (typeof topLevelUrl === 'string') return topLevelUrl;
		}

		const data = response?.data;

		if (typeof data === 'string') return data;

		if (typeof data !== 'object' || data === null) return undefined;

		const objectData = data as Record<string, unknown>;
		const candidate =
			objectData.url ??
			objectData.pay_url ??
			objectData.payment_url ??
			objectData.checkout_url ??
			objectData.qr_code;

		if (typeof candidate === 'string') return candidate;
	}

	function extractPayFields(
		response: { data?: unknown } | undefined
	): Record<string, string> | undefined {
		const data = response?.data;

		if (typeof data !== 'object' || data === null || Array.isArray(data)) return undefined;

		const fields = Object.entries(data as Record<string, unknown>).reduce<Record<string, string>>(
			(accumulator, [key, value]) => {
				if (value === undefined || value === null) return accumulator;
				accumulator[key] = String(value);
				return accumulator;
			},
			{}
		);

		return Object.keys(fields).length === 0 ? undefined : fields;
	}

	function buildPayUrl(url: string, fields: Record<string, string>) {
		const targetUrl = new URL(url, window.location.origin);

		for (const [key, value] of Object.entries(fields)) {
			targetUrl.searchParams.set(key, value);
		}

		return targetUrl.toString();
	}

	$effect(() => {
		const amount = parseAmount(amountInput);

		quoteError = '';
		quotedAmount = undefined;

		if (amount === undefined) return;

		const timer = window.setTimeout(async () => {
			const response = await quoteAmount({ amount });
			const nextQuotedAmount = extractQuotedAmount(response);

			if (nextQuotedAmount === undefined) {
				quoteError = 'Failed to read the payable amount from /api/user/amount.';
				return;
			}

			quotedAmount = nextQuotedAmount;
		}, 300);

		return () => window.clearTimeout(timer);
	});

	async function handlePay() {
		payError = '';

		const amount = parseAmount(amountInput);
		if (amount === undefined) {
			payError = 'Enter a valid amount first.';
			return;
		}

		const response = await pay({ amount, payment_method: 'alipay' });
		const payUrl = extractPayUrl(response);
		const payFields = extractPayFields(response);

		if (payUrl === undefined) {
			payError = 'Failed to start payment from /api/user/pay.';
			return;
		}

		if (payFields !== undefined) {
			window.open(buildPayUrl(payUrl, payFields), '_blank', 'noopener,noreferrer');
			return;
		}

		window.open(payUrl, '_blank', 'noopener,noreferrer');
	}
</script>

<div class="mx-auto flex w-full max-w-xl flex-col gap-5">
	<div class="rounded-xl border border-outline bg-input/40 p-4">
		<div class="text-sm text-text/70">Remaining quota</div>
		<div class="mt-2 text-2xl font-semibold text-text">
			{#if remainQuota === undefined}
				Loading...
			{:else}
				{remainQuota.toLocaleString()}
			{/if}
		</div>
		<div class="mt-1 text-sm text-text/70">
			{#if remainMoney === undefined}
				Waiting for /api/data/self
			{:else}
				About {formatMoney(remainMoney)} yuan left
			{/if}
		</div>
	</div>

	<div class="rounded-xl border border-outline p-4">
		<div class="mb-4 text-lg">Pay</div>
		<div class="space-y-4">
			<div>
				<Input
					id="pay-amount"
					type="number"
					min="1"
					step="1"
					bind:value={amountInput}
					placeholder="Enter yuan amount"
				>
					Pay amount (yuan)
				</Input>
			</div>

			<div class="rounded-lg border border-dashed border-outline px-3 py-2 text-sm text-text/80">
				{#if String(amountInput).length === 0}
					Enter an amount to fetch the actual payable money.
				{:else if isQuotePending()}
					Fetching actual payable amount...
				{:else if quotedAmount !== undefined}
					Actual need pay: {quotedAmount}
				{:else if String(amountInput).length > 0}
					Enter a whole-number yuan amount.
				{:else}
					Waiting for /api/user/amount
				{/if}
			</div>

			<Button
				class="w-full px-4 py-2"
				disabled={isPayPending() || isQuotePending()}
				onclick={handlePay}
			>
				{isPayPending() ? 'Starting payment...' : 'Pay'}
			</Button>
		</div>
	</div>

	{#if quoteError || isQuoteError()}
		<Warning>{quoteError || 'Unable to fetch the actual payable amount.'}</Warning>
	{/if}

	{#if payError || isPayError()}
		<Warning>{payError || 'Unable to start payment.'}</Warning>
	{/if}
</div>
