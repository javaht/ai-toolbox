import React from 'react';
import { Button, Card, Dropdown, Space, Switch, Tag, Typography, message } from 'antd';
import {
  ApiOutlined,
  CheckCircleOutlined,
  CheckOutlined,
  CopyOutlined,
  DeleteOutlined,
  EditOutlined,
  EyeOutlined,
  MoreOutlined,
} from '@ant-design/icons';
import type { MenuProps } from 'antd';
import { useTranslation } from 'react-i18next';
import type { ClaudeDesktopProvider } from '@/types/claude';
import ProviderConnectivityStatus from '@/features/coding/shared/providerConnectivity/ProviderConnectivityStatus';
import type { ProviderConnectivityStatusItem } from '@/components/common/ProviderCard/types';

const { Text } = Typography;

interface ClaudeProviderCardProps {
  provider: ClaudeDesktopProvider;
  onEdit: (provider: ClaudeDesktopProvider) => void;
  onCopy: (provider: ClaudeDesktopProvider) => void;
  onDelete: (provider: ClaudeDesktopProvider) => void;
  onApply: (provider: ClaudeDesktopProvider) => void;
  onPreview: (provider: ClaudeDesktopProvider) => void;
  onTest: (provider: ClaudeDesktopProvider) => void;
  onToggleDisabled: (provider: ClaudeDesktopProvider, isDisabled: boolean) => void;
  connectivityStatus?: ProviderConnectivityStatusItem;
}

const maskApiKey = (value: string): string => {
  const trimmed = value.trim();
  if (!trimmed) return '';
  if (trimmed.length <= 12) return '••••••';
  return `${trimmed.slice(0, 6)}...${trimmed.slice(-6)}`;
};

const ClaudeProviderCard: React.FC<ClaudeProviderCardProps> = ({
  provider,
  onEdit,
  onCopy,
  onDelete,
  onApply,
  onPreview,
  onTest,
  onToggleDisabled,
  connectivityStatus,
}) => {
  const { t } = useTranslation();
  const modelNames = React.useMemo(
    () => (provider.inferenceModels || [])
      .map((model) => model.name.trim())
      .filter(Boolean),
    [provider.inferenceModels],
  );
  const canRunConnectivityTest = Boolean(
    provider.inferenceGatewayBaseUrl.trim()
    && provider.inferenceGatewayApiKey.trim()
    && modelNames.length > 0
    && !provider.isDisabled,
  );

  const handleToggleDisabled = (checked: boolean) => {
    if (provider.isApplied && !checked) {
      message.warning(t('common.disableAppliedConfigWarning'));
      return;
    }
    onToggleDisabled(provider, !checked);
  };

  const menuItems: MenuProps['items'] = [
    {
      key: 'toggle',
      label: (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12 }}>
          <span>{t('common.enable')}</span>
          <Switch checked={!provider.isDisabled} onChange={handleToggleDisabled} size="small" />
        </div>
      ),
    },
    {
      key: 'preview',
      label: t('common.preview'),
      icon: <EyeOutlined />,
      onClick: () => onPreview(provider),
    },
    {
      key: 'edit',
      label: t('common.edit'),
      icon: <EditOutlined />,
      onClick: () => onEdit(provider),
    },
    {
      key: 'copy',
      label: t('common.copy'),
      icon: <CopyOutlined />,
      onClick: () => onCopy(provider),
    },
    {
      type: 'divider',
    },
    {
      key: 'delete',
      label: t('common.delete'),
      icon: <DeleteOutlined />,
      danger: true,
      onClick: () => onDelete(provider),
    },
  ];

  return (
    <Card
      size="small"
      style={{
        marginBottom: 12,
        borderColor: provider.isApplied ? '#1890ff' : 'var(--color-border-card)',
        backgroundColor: provider.isApplied ? 'var(--color-bg-selected)' : undefined,
        opacity: provider.isDisabled ? 0.62 : 1,
      }}
      styles={{ body: { padding: 16 } }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12 }}>
        <Space direction="vertical" size={4} style={{ minWidth: 0 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <ProviderConnectivityStatus item={connectivityStatus} />
            <Text strong>{provider.name}</Text>
            <Tag>{provider.inferenceProvider || 'gateway'}</Tag>
            {provider.isApplied ? (
              <Tag color="green" icon={<CheckCircleOutlined />}>
                {t('claude.provider.applied')}
              </Tag>
            ) : null}
            {provider.isDisabled ? <Tag color="default">{t('claude.provider.disabled')}</Tag> : null}
          </div>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {provider.inferenceGatewayBaseUrl || t('common.notSet')}
          </Text>
          <Text type="secondary" style={{ fontSize: 12, fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace' }}>
            {maskApiKey(provider.inferenceGatewayApiKey) || t('common.apiKeyMissing')}
          </Text>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t('claude.provider.models')}:
            </Text>
            {modelNames.length > 0 ? (
              modelNames.map((modelName) => (
                <Text code key={modelName} style={{ fontSize: 12 }}>
                  {modelName}
                </Text>
              ))
            ) : (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t('common.modelMissing')}
              </Text>
            )}
            <Text type="secondary" style={{ fontSize: 12 }}>|</Text>
            <Button
              type="text"
              size="small"
              icon={<ApiOutlined />}
              disabled={!canRunConnectivityTest}
              onClick={() => onTest(provider)}
              style={{ fontSize: 12, padding: '0 4px', height: 'auto', flexShrink: 0 }}
            >
              {t('opencode.connectivity.button')}
            </Button>
          </div>
          {provider.notes ? (
            <Text type="secondary" style={{ fontSize: 12 }}>
              {provider.notes}
            </Text>
          ) : null}
        </Space>

        <Space size={4} style={{ flexShrink: 0 }}>
          {!provider.isApplied ? (
            <Button
              type="primary"
              size="small"
              icon={<CheckOutlined />}
              disabled={provider.isDisabled}
              onClick={() => onApply(provider)}
            >
              {t('claude.provider.apply')}
            </Button>
          ) : null}
          <Dropdown menu={{ items: menuItems }} trigger={['click']}>
            <Button type="text" icon={<MoreOutlined />} />
          </Dropdown>
        </Space>
      </div>
    </Card>
  );
};

export default ClaudeProviderCard;
