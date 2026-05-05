import React from 'react';
import { AutoComplete, Button, Form, Input, message, Modal, Radio, Space } from 'antd';
import { CloudDownloadOutlined, EyeInvisibleOutlined, EyeOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/stores';
import type {
  ClaudeDesktopProvider,
  ClaudeDesktopProviderFormValues,
} from '@/types/claude';

const { TextArea } = Input;
const MODEL_FIELD_NAMES = ['model', 'haikuModel', 'sonnetModel', 'opusModel', 'reasoningModel'] as const;

interface FetchedModel {
  id: string;
  name?: string;
}

interface FetchModelsResponse {
  models: FetchedModel[];
  total: number;
}

const normalizeGatewayBaseUrl = (value: string) => {
  let base = value.trim().replace(/\/$/, '');
  if (base.endsWith('/v1')) {
    base = base.slice(0, -3);
  }
  return base;
};

const toModelName = (value: unknown) => (
  typeof value === 'string' ? value.trim() : ''
);

const getErrorMessage = (error: unknown) => (
  error instanceof Error ? error.message : String(error)
);

const shouldFallbackToOpenaiCompat = (error: unknown) => {
  const message = getErrorMessage(error);
  return message.includes('Failed to parse Anthropic response') ||
    message.includes('error decoding response body');
};

interface ClaudeProviderFormModalProps {
  open: boolean;
  provider?: ClaudeDesktopProvider | null;
  isCopy?: boolean;
  onCancel: () => void;
  onSubmit: (values: ClaudeDesktopProviderFormValues) => Promise<void>;
}

const ClaudeProviderFormModal: React.FC<ClaudeProviderFormModalProps> = ({
  open,
  provider,
  isCopy = false,
  onCancel,
  onSubmit,
}) => {
  const { t } = useTranslation();
  const language = useAppStore((state) => state.language);
  const [form] = Form.useForm();
  const [loading, setLoading] = React.useState(false);
  const [showApiKey, setShowApiKey] = React.useState(false);
  const [fetchedModels, setFetchedModels] = React.useState<FetchedModel[]>([]);
  const [loadingModels, setLoadingModels] = React.useState(false);
  const [fetchApiType, setFetchApiType] = React.useState<'openai_compat' | 'native'>('openai_compat');
  const labelCol = { span: language === 'zh-CN' ? 4 : 6 };
  const wrapperCol = { span: 20 };
  const isEdit = Boolean(provider && !isCopy);

  React.useEffect(() => {
    if (!open) {
      setFetchedModels([]);
      setShowApiKey(false);
      return;
    }

    if (provider) {
      const modelNames = (provider.inferenceModels || [])
        .map((model) => toModelName(model.name))
        .filter(Boolean);

      form.setFieldsValue({
        name: isCopy ? `${provider.name} Copy` : provider.name,
        inferenceProvider: provider.inferenceProvider || 'gateway',
        inferenceGatewayBaseUrl: provider.inferenceGatewayBaseUrl,
        inferenceGatewayApiKey: provider.inferenceGatewayApiKey,
        model: modelNames[0],
        haikuModel: modelNames[1],
        sonnetModel: modelNames[2],
        opusModel: modelNames[3],
        reasoningModel: modelNames[4],
        notes: provider.notes || '',
      });
    } else {
      form.resetFields();
      form.setFieldsValue({
        inferenceProvider: 'gateway',
      });
    }
    setFetchedModels([]);
    setFetchApiType('openai_compat');
  }, [form, isCopy, open, provider]);

  const modelOptions = React.useMemo(() => {
    const seenIds = new Set<string>();

    return fetchedModels.reduce<{ label: string; value: string }[]>((options, model) => {
      if (!model.id || seenIds.has(model.id)) {
        return options;
      }

      seenIds.add(model.id);
      const name = model.name || model.id;
      options.push({
        label: name && name !== model.id ? `${name} (${model.id})` : model.id,
        value: model.id,
      });
      return options;
    }, []);
  }, [fetchedModels]);

  const requestModels = async (
    base: string,
    apiKey: string | undefined,
    apiType: 'openai_compat' | 'native',
  ) => {
    const customUrl = `${base}/v1/models`;

    return invoke<FetchModelsResponse>('fetch_provider_models', {
      request: {
        baseUrl: `${base}/v1`,
        apiKey: apiKey || undefined,
        apiType,
        sdkType: '@ai-sdk/anthropic',
        customUrl,
      },
    });
  };

  const handleFetchModels = async () => {
    const baseUrl = form.getFieldValue('inferenceGatewayBaseUrl')?.trim();
    const apiKey = form.getFieldValue('inferenceGatewayApiKey')?.trim();

    if (!baseUrl) {
      message.warning(t('claude.fetchModels.baseUrlRequired'));
      return;
    }

    const base = normalizeGatewayBaseUrl(baseUrl);

    setLoadingModels(true);
    try {
      let usedFallback = false;
      let response: FetchModelsResponse;

      try {
        response = await requestModels(base, apiKey, fetchApiType);
      } catch (error) {
        if (fetchApiType !== 'native' || !shouldFallbackToOpenaiCompat(error)) {
          throw error;
        }

        response = await requestModels(base, apiKey, 'openai_compat');
        setFetchApiType('openai_compat');
        usedFallback = true;
      }

      setFetchedModels(response.models);
      if (response.models.length > 0) {
        message.success(
          t(usedFallback ? 'claude.fetchModels.fallbackSuccess' : 'claude.fetchModels.success', {
            count: response.models.length,
          }),
        );
      } else {
        message.info(t('claude.fetchModels.noModels'));
      }
    } catch (error) {
      console.error('Failed to fetch Claude Desktop provider models:', error);
      message.error(t('claude.fetchModels.failed'));
    } finally {
      setLoadingModels(false);
    }
  };

  const handleSubmit = async () => {
    const values = await form.validateFields();
    setLoading(true);
    try {
      await onSubmit({
        name: values.name.trim(),
        inferenceProvider: 'gateway',
        inferenceGatewayBaseUrl: normalizeGatewayBaseUrl(values.inferenceGatewayBaseUrl),
        inferenceGatewayApiKey: values.inferenceGatewayApiKey.trim(),
        inferenceModels: MODEL_FIELD_NAMES
          .map((fieldName) => toModelName(values[fieldName]))
          .filter(Boolean)
          .map((name) => ({ name })),
        notes: values.notes?.trim() || undefined,
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title={isEdit ? t('claude.provider.editProvider') : t('claude.provider.addProvider')}
      open={open}
      onCancel={onCancel}
      onOk={handleSubmit}
      confirmLoading={loading}
      width={720}
      destroyOnHidden
    >
      <Form form={form} labelCol={labelCol} wrapperCol={wrapperCol}>
        <Form.Item label={t('claude.provider.mode')}>
          <Radio.Group value="custom">
            <Radio.Button value="custom">{t('claude.provider.modeCustom')}</Radio.Button>
          </Radio.Group>
        </Form.Item>

        <Form.Item
          name="name"
          label={t('claude.provider.name')}
          rules={[{ required: true, message: t('claude.provider.nameRequired') }]}
        >
          <Input placeholder={t('claude.provider.namePlaceholder')} />
        </Form.Item>

        <Form.Item
          name="inferenceGatewayBaseUrl"
          label={t('claude.provider.baseUrl')}
          rules={[{ required: true, message: t('claude.provider.baseUrlRequired') }]}
        >
          <Input placeholder={t('claude.provider.baseUrlPlaceholder')} />
        </Form.Item>

        <Form.Item
          name="inferenceGatewayApiKey"
          label={t('claude.provider.apiKey')}
          rules={[{ required: true, message: t('claude.provider.apiKeyRequired') }]}
        >
          <Input
            type={showApiKey ? 'text' : 'password'}
            placeholder={t('claude.provider.apiKeyPlaceholder')}
            addonAfter={
              <Button
                type="text"
                size="small"
                icon={showApiKey ? <EyeInvisibleOutlined /> : <EyeOutlined />}
                onClick={() => setShowApiKey((value) => !value)}
                style={{ margin: -7 }}
              />
            }
          />
        </Form.Item>

        <Form.Item wrapperCol={{ offset: labelCol.span, span: wrapperCol.span }}>
          <Space size="middle" style={{ width: '100%' }} wrap>
            <Radio.Group
              value={fetchApiType}
              onChange={(event) => setFetchApiType(event.target.value)}
              size="small"
            >
              <Radio value="openai_compat">{t('claude.fetchModels.openaiCompat')}</Radio>
              <Radio value="native">{t('claude.fetchModels.native')}</Radio>
            </Radio.Group>
            <Button
              type="default"
              icon={<CloudDownloadOutlined />}
              loading={loadingModels}
              onClick={handleFetchModels}
            >
              {t('claude.fetchModels.button')}
            </Button>
            {fetchedModels.length > 0 && (
              <span style={{ color: '#52c41a' }}>
                {t('claude.fetchModels.loaded', { count: fetchedModels.length })}
              </span>
            )}
          </Space>
        </Form.Item>

        <Form.Item name="model" label={t('claude.model.defaultModel')}>
          <AutoComplete
            options={modelOptions}
            placeholder={t('claude.model.defaultModelPlaceholder')}
            style={{ width: '100%' }}
            filterOption={(inputValue, option) =>
              (option?.label.toLowerCase().includes(inputValue.toLowerCase()) ||
                option?.value.toLowerCase().includes(inputValue.toLowerCase())) ?? false
            }
          />
        </Form.Item>

        <Form.Item name="haikuModel" label={t('claude.model.haikuModel')}>
          <AutoComplete
            options={modelOptions}
            placeholder={t('claude.model.haikuModelPlaceholder')}
            style={{ width: '100%' }}
            filterOption={(inputValue, option) =>
              (option?.label.toLowerCase().includes(inputValue.toLowerCase()) ||
                option?.value.toLowerCase().includes(inputValue.toLowerCase())) ?? false
            }
          />
        </Form.Item>

        <Form.Item name="sonnetModel" label={t('claude.model.sonnetModel')}>
          <AutoComplete
            options={modelOptions}
            placeholder={t('claude.model.sonnetModelPlaceholder')}
            style={{ width: '100%' }}
            filterOption={(inputValue, option) =>
              (option?.label.toLowerCase().includes(inputValue.toLowerCase()) ||
                option?.value.toLowerCase().includes(inputValue.toLowerCase())) ?? false
            }
          />
        </Form.Item>

        <Form.Item name="opusModel" label={t('claude.model.opusModel')}>
          <AutoComplete
            options={modelOptions}
            placeholder={t('claude.model.opusModelPlaceholder')}
            style={{ width: '100%' }}
            filterOption={(inputValue, option) =>
              (option?.label.toLowerCase().includes(inputValue.toLowerCase()) ||
                option?.value.toLowerCase().includes(inputValue.toLowerCase())) ?? false
            }
          />
        </Form.Item>

        <Form.Item name="reasoningModel" label={t('claude.model.reasoningModel')}>
          <AutoComplete
            options={modelOptions}
            placeholder={t('claude.model.reasoningModelPlaceholder')}
            style={{ width: '100%' }}
            filterOption={(inputValue, option) =>
              (option?.label.toLowerCase().includes(inputValue.toLowerCase()) ||
                option?.value.toLowerCase().includes(inputValue.toLowerCase())) ?? false
            }
          />
        </Form.Item>

        <Form.Item name="notes" label={t('claude.provider.notes')}>
          <TextArea rows={3} placeholder={t('claude.provider.notesPlaceholder')} />
        </Form.Item>
      </Form>
    </Modal>
  );
};

export default ClaudeProviderFormModal;
