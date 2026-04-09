import { describe, expect, it, vi } from 'vitest';

import {
	createExtractedPdfTextFile,
	prepareUploadFile,
	type PdfModelCapability,
	shouldExtractPdfText
} from './pdf-upload';

describe('shouldExtractPdfText', () => {
	it('returns true for pdf files when model lacks pdf support', () => {
		const file = new File(['dummy'], 'report.pdf', { type: 'application/pdf' });

		expect(shouldExtractPdfText(file, undefined)).toBe(true);
		expect(
			shouldExtractPdfText(file, {
				native_file_input: false,
				ocr_file_input: false
			})
		).toBe(true);
	});

	it('returns false for non-pdf files and pdf-capable models', () => {
		const textFile = new File(['hello'], 'notes.txt', { type: 'text/plain' });
		const pdfFile = new File(['dummy'], 'report.pdf', { type: 'application/pdf' });

		expect(shouldExtractPdfText(textFile, undefined)).toBe(false);
		expect(
			shouldExtractPdfText(pdfFile, {
				native_file_input: true,
				ocr_file_input: false
			})
		).toBe(false);
		expect(
			shouldExtractPdfText(pdfFile, {
				native_file_input: false,
				ocr_file_input: true
			})
		).toBe(false);
	});
});

describe('createExtractedPdfTextFile', () => {
	it('creates a text file with pdf metadata in the content', async () => {
		const file = new File(['dummy'], 'report.pdf', { type: 'application/pdf' });
		const extracted = createExtractedPdfTextFile(file, 'Page one\nPage two');

		expect(extracted.name).toBe('report.pdf.txt');
		expect(extracted.type).toBe('text/plain');
		expect(await extracted.text()).toContain('Page one');
	});
});

describe('prepareUploadFile', () => {
	it('replaces pdf with extracted text for text-only models', async () => {
		const file = new File(['dummy'], 'report.pdf', { type: 'application/pdf' });
		const extractText = vi.fn(async () => 'Converted text');

		const result = await prepareUploadFile(file, undefined, extractText);

		expect(extractText).toHaveBeenCalledWith(file);
		expect(result.name).toBe('report.pdf.txt');
		expect(await result.text()).toContain('Converted text');
	});

	it('keeps original pdf when model supports pdf input', async () => {
		const file = new File(['dummy'], 'report.pdf', { type: 'application/pdf' });
		const capability: PdfModelCapability = {
			native_file_input: true,
			ocr_file_input: false
		};
		const extractText = vi.fn(async () => 'Converted text');

		const result = await prepareUploadFile(file, capability, extractText);

		expect(extractText).not.toHaveBeenCalled();
		expect(result).toBe(file);
	});
});
