import { createMutation, createQueryEffect, type MutationResult } from './state';

export interface NewApiEnvelope<T> {
	message?: string;
	data?: T;
	success?: boolean;
}

export interface SelfQuotaData {
	quota?: number;
	used_quota?: number;
	remain_quota?: number;
	unlimited_quota?: boolean;
}

export interface PayAmountReq {
	amount: number;
}

export interface PayReq extends PayAmountReq {
	payment_method: string;
}

export interface PayFormFields {
	[key: string]: string | number | boolean | null | undefined;
}

export interface PayResponse {
	data?: PayFormFields | string;
	message?: string;
	success?: boolean;
	url?: string;
}

let selfQuota = $state<NewApiEnvelope<SelfQuotaData> | undefined>(undefined);

export function useSelfQuotaQueryEffect() {
	createQueryEffect<Record<string, never>, NewApiEnvelope<SelfQuotaData>>({
		path: 'data/self',
		body: {},
		method: 'GET',
		staleTime: 300000,
		updateData: (data) => {
			selfQuota = data;
		}
	});
}

export function getSelfQuota(): NewApiEnvelope<SelfQuotaData> | undefined {
	return selfQuota;
}

export function getPayAmount(): MutationResult<PayAmountReq, NewApiEnvelope<unknown>> {
	return createMutation({
		path: 'user/amount'
	});
}

export function startPay(): MutationResult<PayReq, PayResponse> {
	return createMutation({
		path: 'user/pay'
	});
}
