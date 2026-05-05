import React from 'react';
import { Button, Collapse, Empty, message, Modal, Space, Spin, Switch, Typography } from 'antd';
import {
  AppstoreOutlined,
  DatabaseOutlined,
  EditOutlined,
  EllipsisOutlined,
  FolderOpenOutlined,
  MessageOutlined,
  PlusOutlined,
  SyncOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type {
  ClaudeDesktopProvider,
  ClaudeDesktopProviderFormValues,
  ClaudeDesktopProviderInput,
} from '@/types/claude';
import type { OpenCodeDiagnosticsConfig, OpenCodeFavoriteProvider } from '@/services/opencodeApi';
import {
  listFavoriteProviders,
  upsertFavoriteProvider,
} from '@/services/opencodeApi';
import {
  applyClaudeDesktopConfig,
  createClaudeDesktopProvider,
  deleteClaudeDesktopProvider,
  getClaudeDesktopCommonConfig,
  getClaudeDesktopConfigPath,
  getClaudeDesktopRootPathInfo,
  listClaudeDesktopProviders,
  saveClaudeDesktopCommonConfig,
  toggleClaudeDesktopProviderDisabled,
  updateClaudeDesktopProvider,
} from '@/services/claudeApi';
import { refreshTrayMenu } from '@/services/appApi';
import JsonPreviewModal from '@/components/common/JsonPreviewModal';
import RootDirectoryModal from '@/features/coding/shared/RootDirectoryModal';
import useRootDirectoryConfig from '@/features/coding/shared/useRootDirectoryConfig';
import SectionSidebarLayout, {
  type SidebarSectionMarker,
} from '@/components/layout/SectionSidebarLayout/SectionSidebarLayout';
import { useSettingsStore } from '@/stores';
import { SessionManagerPanel } from '@/features/coding/shared/sessionManager';
import ProviderConnectivityTestModal, {
  buildClaudeDesktopProviderConnectivityInfo,
  type ProviderConnectivityInfo,
} from '@/features/coding/shared/providerConnectivity/ProviderConnectivityTestModal';
import {
  buildProviderConnectivityBatchTarget,
  runProviderConnectivityBatch,
} from '@/features/coding/shared/providerConnectivity/batchTest';
import type { ProviderConnectivityStatusItem } from '@/components/common/ProviderCard/types';
import {
  buildFavoriteProviderOptions,
  buildFavoriteProviderStorageKey,
  findDefaultTestModelIdForProvider,
  findDiagnosticsForProvider,
  isFavoriteProviderForSource,
  mergeDiagnosticsIntoFavoriteProviders,
} from '@/features/coding/shared/favoriteProviders';
import ClaudeProviderCard from '../components/ClaudeProviderCard';
import ClaudeCommonConfigModal from '../components/ClaudeCommonConfigModal';
import ClaudeProviderFormModal from '../components/ClaudeProviderFormModal';

const { Title, Text } = Typography;
const DEFAULT_COMMON_CONFIG = '{}';

const buildProviderInput = (
  values: ClaudeDesktopProviderFormValues,
  sortIndex?: number,
): ClaudeDesktopProviderInput => ({
  name: values.name,
  inferenceProvider: values.inferenceProvider || 'gateway',
  inferenceGatewayBaseUrl: values.inferenceGatewayBaseUrl,
  inferenceGatewayApiKey: values.inferenceGatewayApiKey,
  inferenceModels: values.inferenceModels,
  notes: values.notes,
  sortIndex,
});

const buildPreviewData = (provider: ClaudeDesktopProvider) => ({
  inferenceProvider: provider.inferenceProvider,
  inferenceGatewayBaseUrl: provider.inferenceGatewayBaseUrl,
  inferenceGatewayApiKey: provider.inferenceGatewayApiKey ? '******' : '',
  inferenceModels: provider.inferenceModels,
});

const buildClaudeDesktopFavoriteProviderConfig = (provider: ClaudeDesktopProvider) => {
  const connectivityInfo = buildClaudeDesktopProviderConnectivityInfo(provider);
  return buildFavoriteProviderOptions(connectivityInfo.providerConfig, {
    name: provider.name,
    inferenceProvider: provider.inferenceProvider,
    inferenceGatewayBaseUrl: provider.inferenceGatewayBaseUrl,
    inferenceModels: provider.inferenceModels,
    notes: provider.notes,
  });
};

const ClaudePage: React.FC = () => {
  const { t } = useTranslation();
  const { sidebarHiddenByPage, setSidebarHidden } = useSettingsStore();
  const [loading, setLoading] = React.useState(false);
  const [configPath, setConfigPath] = React.useState('');
  const [rootPathInfo, setRootPathInfo] = React.useState<{
    path: string;
    source: 'custom' | 'default';
  } | null>(null);
  const [providers, setProviders] = React.useState<ClaudeDesktopProvider[]>([]);
  const [providerModalOpen, setProviderModalOpen] = React.useState(false);
  const [editingProvider, setEditingProvider] = React.useState<ClaudeDesktopProvider | null>(null);
  const [isCopyMode, setIsCopyMode] = React.useState(false);
  const [previewData, setPreviewData] = React.useState<unknown>(null);
  const [providerListCollapsed, setProviderListCollapsed] = React.useState(false);
  const [settingsModalOpen, setSettingsModalOpen] = React.useState(false);
  const [commonConfigModalOpen, setCommonConfigModalOpen] = React.useState(false);
  const [connectivityModalOpen, setConnectivityModalOpen] = React.useState(false);
  const [connectivityInfo, setConnectivityInfo] = React.useState<ProviderConnectivityInfo | null>(null);
  const [connectivityStatuses, setConnectivityStatuses] = React.useState<Record<string, ProviderConnectivityStatusItem>>({});
  const [batchTestingProviders, setBatchTestingProviders] = React.useState(false);
  const [favoriteProviders, setFavoriteProviders] = React.useState<OpenCodeFavoriteProvider[]>([]);
  const [sessionManagerExpandNonce, setSessionManagerExpandNonce] = React.useState(0);
  const sidebarHidden = sidebarHiddenByPage.claude;

  const sidebarSections = React.useMemo<SidebarSectionMarker[]>(
    () => [
      {
        id: 'claude-providers',
        title: t('claude.provider.title'),
        order: 1,
      },
      {
        id: 'claude-session-manager',
        title: t('sessionManager.title'),
        order: 2,
      },
    ],
    [t],
  );

  const loadConfig = React.useCallback(async (silent = false) => {
    setLoading(true);
    try {
      const [path, nextRootPathInfo, providerList] = await Promise.all([
        getClaudeDesktopConfigPath(),
        getClaudeDesktopRootPathInfo(),
        listClaudeDesktopProviders(),
      ]);
      setConfigPath(path);
      setRootPathInfo(nextRootPathInfo);
      setProviders(providerList);
    } catch (error) {
      console.error('Failed to load Claude Desktop config:', error);
      if (!silent) {
        message.error(t('common.error'));
      }
    } finally {
      setLoading(false);
    }
  }, [t]);

  const {
    rootDirectoryModalOpen,
    setRootDirectoryModalOpen,
    getSourceLabel,
    getRootDirectoryModalProps,
    handleSaveRootDirectory,
    handleResetRootDirectory,
  } = useRootDirectoryConfig({
    t,
    translationKeyPrefix: 'claude',
    defaultConfig: DEFAULT_COMMON_CONFIG,
    loadConfig,
    getCommonConfig: getClaudeDesktopCommonConfig,
    saveCommonConfig: saveClaudeDesktopCommonConfig,
  });

  React.useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const loadFavoriteProviders = React.useCallback(async () => {
    try {
      const allFavoriteProviders = await listFavoriteProviders();
      setFavoriteProviders(
        allFavoriteProviders.filter((provider) => isFavoriteProviderForSource('claude', provider)),
      );
    } catch (error) {
      console.error('Failed to load Claude Desktop favorite providers:', error);
    }
  }, []);

  React.useEffect(() => {
    void loadFavoriteProviders();
  }, [loadFavoriteProviders]);

  const openAddModal = () => {
    setEditingProvider(null);
    setIsCopyMode(false);
    setProviderModalOpen(true);
  };

  const handleEdit = (provider: ClaudeDesktopProvider) => {
    setEditingProvider(provider);
    setIsCopyMode(false);
    setProviderModalOpen(true);
  };

  const handleCopy = (provider: ClaudeDesktopProvider) => {
    setEditingProvider(provider);
    setIsCopyMode(true);
    setProviderModalOpen(true);
  };

  const handleSubmitProvider = async (values: ClaudeDesktopProviderFormValues) => {
    const input = buildProviderInput(values, editingProvider?.sortIndex);
    if (editingProvider && !isCopyMode) {
      await updateClaudeDesktopProvider({
        ...editingProvider,
        ...input,
      });
    } else {
      await createClaudeDesktopProvider(input);
    }
    setProviderModalOpen(false);
    setEditingProvider(null);
    setIsCopyMode(false);
    await loadConfig(true);
    await refreshTrayMenu();
    message.success(t('common.success'));
  };

  const handleApply = async (provider: ClaudeDesktopProvider) => {
    await applyClaudeDesktopConfig(provider.id);
    await loadConfig(true);
    await refreshTrayMenu();
    message.success(t('claude.apply.success'));
  };

  const handleDelete = (provider: ClaudeDesktopProvider) => {
    Modal.confirm({
      title: t('claude.provider.confirmDelete', { name: provider.name }),
      okText: t('common.delete'),
      okType: 'danger',
      cancelText: t('common.cancel'),
      onOk: async () => {
        await deleteClaudeDesktopProvider(provider.id);
        await loadConfig(true);
        await refreshTrayMenu();
        message.success(t('common.success'));
      },
    });
  };

  const handleToggleDisabled = async (
    provider: ClaudeDesktopProvider,
    isDisabled: boolean,
  ) => {
    await toggleClaudeDesktopProviderDisabled(provider.id, isDisabled);
    await loadConfig(true);
  };

  const handleTestProvider = (provider: ClaudeDesktopProvider) => {
    setConnectivityInfo(buildClaudeDesktopProviderConnectivityInfo(provider));
    setConnectivityModalOpen(true);
  };

  const handleSaveConnectivityDiagnostics = React.useCallback(async (
    diagnostics: OpenCodeDiagnosticsConfig,
  ) => {
    if (!connectivityInfo) {
      return;
    }

    const targetProvider = providers.find((provider) => provider.id === connectivityInfo.providerId);
    if (!targetProvider) {
      return;
    }

    try {
      const favoriteProvider = await upsertFavoriteProvider(
        buildFavoriteProviderStorageKey('claude', targetProvider.id),
        buildClaudeDesktopFavoriteProviderConfig(targetProvider),
        diagnostics,
      );
      setFavoriteProviders((previousProviders) =>
        mergeDiagnosticsIntoFavoriteProviders(previousProviders, favoriteProvider, 'claude'),
      );
    } catch (error) {
      console.error('Failed to save Claude Desktop connectivity diagnostics:', error);
      message.error(t('common.error'));
    }
  }, [connectivityInfo, providers, t]);

  const handleBatchTestProviders = React.useCallback(async () => {
    if (providers.length === 0) {
      return;
    }

    const testableProviders = providers.filter((provider) => !provider.isDisabled);
    if (testableProviders.length === 0) {
      setConnectivityStatuses({});
      return;
    }

    const targets = testableProviders.map((provider) => {
      const connectivityInfo = buildClaudeDesktopProviderConnectivityInfo(provider);
      if (!provider.inferenceGatewayBaseUrl.trim()) {
        return {
          providerId: provider.id,
          errorMessage: t('common.baseUrlMissing'),
        };
      }

      return buildProviderConnectivityBatchTarget(connectivityInfo, {
        requireBaseUrl: false,
        requireApiKey: true,
        preferredModelId: findDefaultTestModelIdForProvider(
          favoriteProviders,
          'claude',
          provider.id,
        ),
        errorMessages: {
          missingBaseUrl: t('common.baseUrlMissing'),
          missingApiKey: t('common.apiKeyMissing'),
          missingModel: t('common.modelMissing'),
        },
      });
    });

    setConnectivityStatuses(
      Object.fromEntries(
        testableProviders.map((provider) => [
          provider.id,
          { status: 'running' as const },
        ]),
      ),
    );
    setBatchTestingProviders(true);

    try {
      await runProviderConnectivityBatch(targets, (providerId, status) => {
        const nextStatus = status.status === 'success'
          ? {
              ...status,
              tooltipMessage: status.totalMs !== undefined
                ? t('common.connectivityBatchSuccessWithTiming', {
                    model: status.modelId || t('common.notSet'),
                    totalMs: status.totalMs,
                  })
                : t('common.connectivityBatchSuccess', {
                    model: status.modelId || t('common.notSet'),
                  }),
            }
          : status;
        setConnectivityStatuses((previousStatuses) => ({
          ...previousStatuses,
          [providerId]: nextStatus,
        }));
      });
    } catch (error) {
      console.error('Failed to batch test Claude Desktop providers:', error);
      message.error(t('common.error'));
    } finally {
      setBatchTestingProviders(false);
    }
  }, [favoriteProviders, providers, t]);

  const handleOpenFolder = async () => {
    try {
      await revealItemInDir(configPath);
    } catch (error) {
      console.error('Failed to open Claude Desktop config folder:', error);
      message.error(t('common.error'));
    }
  };

  const content = (
    <div style={{ padding: 24 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <div>
          <Title level={4} style={{ margin: 0, display: 'inline-block', marginRight: 8 }}>
            {t('claude.title')}
          </Title>
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t('claude.pageHint')}
          </Text>
        </div>
        <Space size={4}>
          <Button
            type="text"
            icon={<EditOutlined />}
            onClick={() => setRootDirectoryModalOpen(true)}
          >
            {t('claude.rootPathSource.customize')}
          </Button>
          <Button type="text" icon={<FolderOpenOutlined />} onClick={handleOpenFolder}>
            {t('claude.openFolder')}
          </Button>
          <Button type="text" icon={<SyncOutlined />} onClick={() => loadConfig()}>
            {t('claude.refreshConfig')}
          </Button>
          <Button type="text" icon={<EllipsisOutlined />} onClick={() => setSettingsModalOpen(true)}>
            {t('common.moreOptions')}
          </Button>
        </Space>
      </div>

      <div style={{ marginBottom: 18 }}>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {t('claude.configPath')}:
        </Text>{' '}
        <Text code style={{ fontSize: 12 }}>
          {configPath || '-'}
        </Text>
        {rootPathInfo ? (
          <Text type="secondary" style={{ fontSize: 12, marginLeft: 8 }}>
            {getSourceLabel(rootPathInfo)}
          </Text>
        ) : null}
      </div>

      <Spin spinning={loading}>
        <div
          id="claude-providers"
          data-sidebar-section="true"
          data-sidebar-title={t('claude.provider.title')}
        >
          <Collapse
            activeKey={providerListCollapsed ? [] : ['providers']}
            onChange={(keys) => {
              const activeKeys = Array.isArray(keys) ? keys : [keys];
              setProviderListCollapsed(!activeKeys.includes('providers'));
            }}
            items={[
              {
                key: 'providers',
                label: (
                  <Space>
                    <AppstoreOutlined />
                    <Text strong>{t('claude.provider.title')}</Text>
                  </Space>
                ),
                extra: (
                  <Space size={4}>
                    <Button
                      type="link"
                      size="small"
                      style={{ fontSize: 12 }}
                      icon={<ThunderboltOutlined />}
                      loading={batchTestingProviders}
                      onClick={(event) => {
                        event.stopPropagation();
                        void handleBatchTestProviders();
                      }}
                    >
                      {t('common.batchTest')}
                    </Button>
                    <Button
                      type="link"
                      size="small"
                      style={{ fontSize: 12 }}
                      icon={<AppstoreOutlined />}
                      onClick={(event) => {
                        event.stopPropagation();
                        setCommonConfigModalOpen(true);
                      }}
                    >
                      {t('claude.commonConfigButton')}
                    </Button>
                    <Button
                      type="link"
                      icon={<PlusOutlined />}
                      onClick={(event) => {
                        event.stopPropagation();
                        openAddModal();
                      }}
                    >
                      {t('claude.provider.addProvider')}
                    </Button>
                  </Space>
                ),
                children:
                  providers.length > 0 ? (
                    providers.map((provider) => (
                      <ClaudeProviderCard
                        key={provider.id}
                        provider={provider}
                        onEdit={handleEdit}
                        onCopy={handleCopy}
                        onDelete={handleDelete}
                        onApply={handleApply}
                        onPreview={(nextProvider) => setPreviewData(buildPreviewData(nextProvider))}
                        onTest={handleTestProvider}
                        onToggleDisabled={handleToggleDisabled}
                        connectivityStatus={connectivityStatuses[provider.id]}
                      />
                    ))
                  ) : (
                    <Empty description={t('claude.emptyText')}>
                      <Button type="primary" icon={<PlusOutlined />} onClick={openAddModal}>
                        {t('claude.provider.addProvider')}
                      </Button>
                    </Empty>
                  ),
              },
            ]}
          />
        </div>
      </Spin>

      <div
        id="claude-session-manager"
        data-sidebar-section="true"
        data-sidebar-title={t('sessionManager.title')}
        style={{ marginTop: 16 }}
      >
        <SessionManagerPanel tool="claude" expandNonce={sessionManagerExpandNonce} />
      </div>

      <ClaudeProviderFormModal
        open={providerModalOpen}
        provider={editingProvider}
        isCopy={isCopyMode}
        onCancel={() => {
          setProviderModalOpen(false);
          setEditingProvider(null);
          setIsCopyMode(false);
        }}
        onSubmit={handleSubmitProvider}
      />

      <ClaudeCommonConfigModal
        open={commonConfigModalOpen}
        onCancel={() => setCommonConfigModalOpen(false)}
        onSuccess={() => {
          setCommonConfigModalOpen(false);
        }}
      />

      <ProviderConnectivityTestModal
        open={connectivityModalOpen}
        connectivityInfo={connectivityInfo}
        diagnostics={
          connectivityInfo
            ? findDiagnosticsForProvider(favoriteProviders, 'claude', connectivityInfo.providerId)
            : undefined
        }
        onSaveDiagnostics={handleSaveConnectivityDiagnostics}
        onCancel={() => {
          setConnectivityModalOpen(false);
          setConnectivityInfo(null);
        }}
      />

      <JsonPreviewModal
        open={previewData !== null}
        onClose={() => setPreviewData(null)}
        title={t('claude.provider.previewConfig')}
        data={previewData}
      />

      {rootDirectoryModalOpen ? (
        <RootDirectoryModal
          open={rootDirectoryModalOpen}
          {...getRootDirectoryModalProps(rootPathInfo)}
          onCancel={() => setRootDirectoryModalOpen(false)}
          onSubmit={handleSaveRootDirectory}
          onReset={handleResetRootDirectory}
        />
      ) : null}

      <Modal
        title={t('claude.settings.title')}
        open={settingsModalOpen}
        onCancel={() => setSettingsModalOpen(false)}
        footer={null}
        width={520}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 16 }}>
          <Text>{t('common.showSidebar')}</Text>
          <Switch
            checked={!sidebarHidden}
            onChange={(checked) => setSidebarHidden('claude', !checked)}
          />
        </div>
      </Modal>
    </div>
  );

  return (
    <SectionSidebarLayout
      sidebarTitle={t('claude.title')}
      sections={sidebarSections}
      sidebarHidden={sidebarHidden}
      getIcon={(id) => {
        switch (id) {
          case 'claude-providers':
            return <DatabaseOutlined />;
          case 'claude-session-manager':
            return <MessageOutlined />;
          default:
            return null;
        }
      }}
      onSectionSelect={(id) => {
        if (id === 'claude-providers') {
          setProviderListCollapsed(false);
        } else if (id === 'claude-session-manager') {
          setSessionManagerExpandNonce((value) => value + 1);
        }
      }}
    >
      {content}
    </SectionSidebarLayout>
  );
};

export default ClaudePage;
