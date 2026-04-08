<script lang="ts">
	import { goto } from '$app/navigation';
	import { token } from '$lib/store';

	$effect(() => {
		if ($token) goto('/chat/new');
	});
</script>

<div class="flex h-screen flex-col items-center justify-center bg-login-bg">
	<h1 class="mb-4 text-center text-3xl md:text-5xl">Welcome to Llumen</h1>
	<p class="mb-6 text-center text-lg text-text/80">
		Sign in through llumen to keep chat history local.
	</p>
	{#if $token}
		<a href="/chat/new" class="text-2xl">Go to chat</a>
	{:else}
		<div class="flex flex-col gap-3 text-center text-2xl">
			<a
				href="/login"
				class="rounded-md border border-outline px-6 py-3 hover:bg-primary hover:text-text-hover"
				>Login</a
			>
			<a
				href="/register"
				class="rounded-md border border-outline px-6 py-3 hover:bg-primary hover:text-text-hover"
				>Register</a
			>
		</div>
	{/if}
</div>
