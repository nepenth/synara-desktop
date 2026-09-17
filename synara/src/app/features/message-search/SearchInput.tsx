import React, { FormEventHandler, RefObject } from 'react';
import { Box, Text, Input, Icon, Icons, Spinner, Chip, config } from 'folds';
import * as depthCss from '../../styles/Depth.css';

type SearchProps = {
  active?: boolean;
  loading?: boolean;
  disabled?: boolean;
  disabledReason?: string;
  searchInputRef: RefObject<HTMLInputElement | null>;
  onSearch: (term: string) => void;
  onReset: () => void;
};
export function SearchInput({
  active,
  loading,
  disabled,
  disabledReason,
  searchInputRef,
  onSearch,
  onReset,
}: SearchProps) {
  const handleSearchSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    if (disabled) return;
    const { searchInput } = evt.target as HTMLFormElement & {
      searchInput: HTMLInputElement;
    };

    const searchTerm = searchInput.value.trim() || undefined;
    if (searchTerm) {
      onSearch(searchTerm);
    }
  };

  return (
    <Box as="form" direction="Column" gap="100" onSubmit={handleSearchSubmit}>
      <span data-spacing-node />
      <Text size="L400">Search</Text>
      <Input
        ref={searchInputRef}
        style={{ paddingRight: config.space.S300 }}
        name="searchInput"
        autoFocus
        size="500"
        variant="Background"
        placeholder="Search for keyword"
        autoComplete="off"
        disabled={disabled}
        before={
          active && loading ? (
            <Spinner variant="Secondary" size="200" />
          ) : (
            <Icon size="200" src={Icons.Search} />
          )
        }
        after={
          active ? (
            <Chip
              key="resetButton"
              className={depthCss.quietInteractiveSurface}
              type="reset"
              variant="Secondary"
              size="400"
              radii="Pill"
              outlined
              after={<Icon size="50" src={Icons.Cross} />}
              onClick={onReset}
            >
              <Text size="B300">Clear</Text>
            </Chip>
          ) : (
            <Chip
              className={depthCss.quietInteractiveSurface}
              type="submit"
              variant="Primary"
              size="400"
              radii="Pill"
              outlined
              disabled={disabled}
            >
              <Text size="B300">Enter</Text>
            </Chip>
          )
        }
      />
      {disabled && disabledReason ? (
        <Text size="T200" style={{ opacity: 0.8 }}>
          {disabledReason}
        </Text>
      ) : null}
    </Box>
  );
}
