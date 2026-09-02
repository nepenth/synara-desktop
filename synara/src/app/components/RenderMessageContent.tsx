import React, { lazy, Suspense } from 'react';
import { ErrorBoundary } from 'react-error-boundary';
import { HTMLReactParserOptions } from 'html-react-parser';
import { Opts } from 'linkifyjs';
import { config } from 'folds';
import {
  AudioContent,
  DownloadFile,
  FileContent,
  ImageContent,
  MAudio,
  MBadEncrypted,
  MEmote,
  MFile,
  MImage,
  MLocation,
  MNotice,
  MText,
  MVideo,
  ReadPdfFile,
  ReadTextFile,
  RenderBody,
  ThumbnailContent,
  UnsupportedContent,
  VideoContent,
} from './message';
import { Image, MediaControl, Video } from './media';
import { ImageViewer } from './image-viewer';
import { PdfViewer } from './Pdf-viewer';
import { TextViewer } from './text-viewer';
import { IImageContent } from '../../types/matrix/common';
import { parseHermesAgentPayload } from '../utils/hermes';
import { useClientConfig } from '../hooks/useClientConfig';
import { detectAgentApprovalPrompt } from '../utils/agentApprovals';
import { AgentApprovalCard, type AgentApprovalTarget } from './agent-approval/AgentApprovalCard';

const HermesAgentCard = lazy(() =>
  import('./hermes/HermesAgentCard').then((module) => ({ default: module.HermesAgentCard }))
);

type RenderMessageContentProps = {
  displayName: string;
  msgType: string;
  ts: number;
  edited?: boolean;
  getContent: <T>() => T;
  mediaAutoLoad?: boolean;
  highlightRegex?: RegExp;
  htmlReactParserOptions: HTMLReactParserOptions;
  linkifyOpts: Opts;
  outlineAttachment?: boolean;
  agentApprovalTarget?: AgentApprovalTarget;
};
export function RenderMessageContent({
  displayName,
  msgType,
  edited,
  getContent,
  mediaAutoLoad,
  highlightRegex,
  htmlReactParserOptions,
  linkifyOpts,
  outlineAttachment,
  agentApprovalTarget,
}: RenderMessageContentProps) {
  const clientConfig = useClientConfig();
  const content = getContent<Record<string, unknown>>();
  const approvalPrompt = detectAgentApprovalPrompt(content);
  if (approvalPrompt && agentApprovalTarget?.coreEligible === true) {
    return <AgentApprovalCard prompt={approvalPrompt} target={agentApprovalTarget} />;
  }

  const hermesPayload = parseHermesAgentPayload(content, clientConfig.agentCards?.contentKeys);
  if (hermesPayload) {
    return (
      <ErrorBoundary fallback={<UnsupportedContent />}>
        <Suspense fallback={null}>
          <HermesAgentCard payload={hermesPayload} />
        </Suspense>
      </ErrorBoundary>
    );
  }

  const renderCaption = () => {
    const imageContent: IImageContent = getContent();
    if (imageContent.filename && imageContent.filename !== imageContent.body) {
      return (
        <MText
          style={{ marginTop: config.space.S200 }}
          edited={edited}
          content={imageContent}
          renderBody={(props) => (
            <RenderBody
              {...props}
              highlightRegex={highlightRegex}
              htmlReactParserOptions={htmlReactParserOptions}
              linkifyOpts={linkifyOpts}
            />
          )}
        />
      );
    }
    return null;
  };

  const renderFile = () => (
    <>
      <MFile
        content={getContent()}
        renderFileContent={({ body, mimeType, info, encInfo, url }) => (
          <FileContent
            body={body}
            mimeType={mimeType}
            renderAsPdfFile={() => (
              <ReadPdfFile
                body={body}
                mimeType={mimeType}
                url={url}
                encInfo={encInfo}
                renderViewer={(p) => <PdfViewer {...p} />}
              />
            )}
            renderAsTextFile={() => (
              <ReadTextFile
                body={body}
                mimeType={mimeType}
                url={url}
                encInfo={encInfo}
                renderViewer={(p) => <TextViewer {...p} />}
              />
            )}
          >
            <DownloadFile body={body} mimeType={mimeType} url={url} encInfo={encInfo} info={info} />
          </FileContent>
        )}
        outlined={outlineAttachment}
      />
      {renderCaption()}
    </>
  );

  if (msgType === 'm.text') {
    return (
      <MText
        edited={edited}
        content={getContent()}
        renderBody={(props) => (
          <RenderBody
            {...props}
            highlightRegex={highlightRegex}
            htmlReactParserOptions={htmlReactParserOptions}
            linkifyOpts={linkifyOpts}
          />
        )}
      />
    );
  }

  if (msgType === 'm.emote') {
    return (
      <MEmote
        displayName={displayName}
        edited={edited}
        content={getContent()}
        renderBody={(props) => (
          <RenderBody
            {...props}
            highlightRegex={highlightRegex}
            htmlReactParserOptions={htmlReactParserOptions}
            linkifyOpts={linkifyOpts}
          />
        )}
      />
    );
  }

  if (msgType === 'm.notice') {
    return (
      <MNotice
        edited={edited}
        content={getContent()}
        renderBody={(props) => (
          <RenderBody
            {...props}
            highlightRegex={highlightRegex}
            htmlReactParserOptions={htmlReactParserOptions}
            linkifyOpts={linkifyOpts}
          />
        )}
      />
    );
  }

  if (msgType === 'm.image') {
    return (
      <>
        <MImage
          content={getContent()}
          renderImageContent={(props) => (
            <ImageContent
              {...props}
              autoPlay={mediaAutoLoad}
              renderImage={(p) => <Image {...p} loading="lazy" />}
              renderViewer={(p) => <ImageViewer {...p} />}
            />
          )}
          outlined={outlineAttachment}
        />
        {renderCaption()}
      </>
    );
  }

  if (msgType === 'm.video') {
    return (
      <>
        <MVideo
          content={getContent()}
          renderAsFile={renderFile}
          renderVideoContent={({ body, info, ...props }) => (
            <VideoContent
              body={body}
              info={info}
              {...props}
              renderThumbnail={
                mediaAutoLoad
                  ? () => (
                      <ThumbnailContent
                        info={info}
                        renderImage={(src) => (
                          <Image alt={body} title={body} src={src} loading="lazy" />
                        )}
                      />
                    )
                  : undefined
              }
              renderVideo={(p) => <Video {...p} />}
            />
          )}
          outlined={outlineAttachment}
        />
        {renderCaption()}
      </>
    );
  }

  if (msgType === 'm.audio') {
    return (
      <>
        <MAudio
          content={getContent()}
          renderAsFile={renderFile}
          renderAudioContent={(props) => (
            <AudioContent {...props} renderMediaControl={(p) => <MediaControl {...p} />} />
          )}
          outlined={outlineAttachment}
        />
        {renderCaption()}
      </>
    );
  }

  if (msgType === 'm.file') {
    return renderFile();
  }

  if (msgType === 'm.location') {
    return <MLocation content={getContent()} />;
  }

  if (msgType === 'm.bad.encrypted') {
    return <MBadEncrypted />;
  }

  return <UnsupportedContent />;
}
