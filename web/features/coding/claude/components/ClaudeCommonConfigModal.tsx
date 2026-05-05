import React from 'react';
import { Alert, Button, message, Modal } from 'antd';
import { useTranslation } from 'react-i18next';
import JsonEditor from '@/components/common/JsonEditor';
import {
  getClaudeDesktopCommonConfig,
  saveClaudeDesktopCommonConfig,
} from '@/services/claudeApi';

interface ClaudeCommonConfigModalProps {
  open: boolean;
  onCancel: () => void;
  onSuccess: () => void;
}

const ClaudeCommonConfigModal: React.FC<ClaudeCommonConfigModalProps> = ({
  open,
  onCancel,
  onSuccess,
}) => {
  const { t } = useTranslation();
  const [loading, setLoading] = React.useState(false);
  const [configValue, setConfigValue] = React.useState<unknown>({});
  const [rootDir, setRootDir] = React.useState<string | null>(null);
  const isValidRef = React.useRef(true);

  React.useEffect(() => {
    if (!open) {
      return;
    }

    const loadConfig = async () => {
      setLoading(true);
      try {
        const config = await getClaudeDesktopCommonConfig();
        setRootDir(config?.rootDir ?? null);
        if (!config?.config) {
          setConfigValue({});
          isValidRef.current = true;
          return;
        }

        const parsedConfig = JSON.parse(config.config) as unknown;
        setConfigValue(parsedConfig);
        isValidRef.current = isPlainObject(parsedConfig);
      } catch (error) {
        console.error('Failed to load Claude Desktop common config:', error);
        const errorMessage = error instanceof Error ? error.message : String(error);
        message.error(errorMessage || t('common.error'));
      } finally {
        setLoading(false);
      }
    };

    void loadConfig();
  }, [open, t]);

  const handleSave = async () => {
    if (!isValidRef.current || !isPlainObject(configValue)) {
      message.error(t('claude.commonConfig.invalidJsonObject'));
      return;
    }

    setLoading(true);
    try {
      await saveClaudeDesktopCommonConfig({
        config: JSON.stringify(configValue, null, 2),
        rootDir,
      });
      message.success(t('common.success'));
      onSuccess();
      onCancel();
    } catch (error) {
      console.error('Failed to save Claude Desktop common config:', error);
      const errorMessage = error instanceof Error ? error.message : String(error);
      message.error(errorMessage || t('common.error'));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      title={t('claude.commonConfig.title')}
      open={open}
      onCancel={onCancel}
      onOk={handleSave}
      confirmLoading={loading}
      width={800}
      okText={t('common.save')}
      cancelText={t('common.cancel')}
      footer={[
        <Button key="cancel" onClick={onCancel} disabled={loading}>
          {t('common.cancel')}
        </Button>,
        <Button key="save" type="primary" onClick={handleSave} loading={loading}>
          {t('common.save')}
        </Button>,
      ]}
    >
      <JsonEditor
        value={configValue}
        onChange={(value, valid) => {
          setConfigValue(value);
          isValidRef.current = valid && isPlainObject(value);
        }}
        mode="text"
        height={400}
        minHeight={200}
        maxHeight={600}
        resizable
        placeholder={`{
  "someSharedOption": true
}`}
      />

      <Alert
        message={t('claude.commonConfig.combinedHint')}
        type="info"
        showIcon
        style={{ marginTop: 12 }}
      />
    </Modal>
  );
};

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export default ClaudeCommonConfigModal;
