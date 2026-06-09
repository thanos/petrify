#!/bin/bash

# Website Mirror Release Script
# This script helps automate the release process

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS] <version>

Options:
    -h, --help          Show this help message
    -p, --patch         Increment patch version (e.g., 1.0.0 -> 1.0.1)
    -m, --minor         Increment minor version (e.g., 1.0.0 -> 1.1.0)
    -M, --major         Increment major version (e.g., 1.0.0 -> 2.0.0)
    -c, --check         Only check current version and suggest next
    -d, --dry-run       Show what would be done without doing it
    -f, --force         Force release even if working directory is not clean

Examples:
    $0 --patch                    # Bump patch version
    $0 --minor                    # Bump minor version
    $0 --major                    # Bump major version
    $0 --check                    # Check current version
    $0 1.2.3                     # Set specific version
    $0 --dry-run --patch         # Show what would happen

EOF
}

# Function to get current version from Cargo.toml
get_current_version() {
    grep '^version = ' Cargo.toml | cut -d '"' -f2
}

# Function to increment version
increment_version() {
    local version=$1
    local increment_type=$2
    
    IFS='.' read -ra VERSION_PARTS <<< "$version"
    local major=${VERSION_PARTS[0]}
    local minor=${VERSION_PARTS[1]}
    local patch=${VERSION_PARTS[2]}
    
    case $increment_type in
        "patch")
            patch=$((patch + 1))
            ;;
        "minor")
            minor=$((minor + 1))
            patch=0
            ;;
        "major")
            major=$((major + 1))
            minor=0
            patch=0
            ;;
    esac
    
    echo "$major.$minor.$patch"
}

# Function to check if working directory is clean
check_working_directory() {
    if [ -n "$(git status --porcelain)" ]; then
        print_warning "Working directory is not clean. Uncommitted changes found:"
        git status --short
        echo
        
        if [ "$FORCE" != "true" ]; then
            print_error "Please commit or stash your changes before releasing."
            print_error "Use --force to override this check."
            exit 1
        else
            print_warning "Proceeding with --force flag..."
        fi
    fi
}

# Function to update version in Cargo.toml
update_version() {
    local new_version=$1
    local cargo_toml="Cargo.toml"
    
    if [ "$DRY_RUN" = "true" ]; then
        print_status "Would update Cargo.toml version to $new_version"
        return
    fi
    
    # Update version in Cargo.toml
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' "s/^version = \".*\"/version = \"$new_version\"/" "$cargo_toml"
    else
        # Linux
        sed -i "s/^version = \".*\"/version = \"$new_version\"/" "$cargo_toml"
    fi
    
    print_success "Updated Cargo.toml version to $new_version"
}

# Function to create git tag
create_git_tag() {
    local version=$1
    local tag="v$version"
    
    if [ "$DRY_RUN" = "true" ]; then
        print_status "Would create git tag: $tag"
        return
    fi
    
    if git tag -l | grep -q "^$tag$"; then
        print_warning "Tag $tag already exists"
        if [ "$FORCE" != "true" ]; then
            print_error "Use --force to overwrite existing tag"
            exit 1
        fi
        git tag -d "$tag" || true
    fi
    
    git tag -a "$tag" -m "Release version $version"
    print_success "Created git tag: $tag"
}

# Function to push changes
push_changes() {
    local version=$1
    local tag="v$version"
    
    if [ "$DRY_RUN" = "true" ]; then
        print_status "Would push changes and tag to remote"
        return
    fi
    
    # Commit version change
    git add Cargo.toml
    git commit -m "chore: bump version to $version"
    
    # Push changes
    git push origin main
    
    # Push tag
    git push origin "$tag"
    
    print_success "Pushed changes and tag to remote"
}

# Function to show release summary
show_release_summary() {
    local version=$1
    local tag="v$version"
    
    cat << EOF

🎉 Release Summary
==================

Version: $version
Tag: $tag
Status: Ready for GitHub Actions

Next Steps:
1. Ensure CRATES_IO_TOKEN is set in GitHub repository secrets.

2. GitHub Actions will automatically:
   - Build binaries for Linux, Windows, and macOS
   - Create a GitHub release
   - Upload release assets
   - Publish petrify to crates.io

2. Monitor the release workflow:
   https://github.com/\$(git config --get remote.origin.url | sed 's/.*github.com[:/]\([^/]*\)\/\([^.]*\).*/\\1\/\\2/')/actions

3. Verify the release:
   https://github.com/\$(git config --get remote.origin.url | sed 's/.*github.com[:/]\([^/]*\)\/\([^.]*\).*/\\1\/\\2/')/releases/tag/$tag

EOF
}

# Main script logic
main() {
    local VERSION=""
    local INCREMENT_TYPE=""
    local CHECK_ONLY=false
    local DRY_RUN=false
    local FORCE=false
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -p|--patch)
                INCREMENT_TYPE="patch"
                shift
                ;;
            -m|--minor)
                INCREMENT_TYPE="minor"
                shift
                ;;
            -M|--major)
                INCREMENT_TYPE="major"
                shift
                ;;
            -c|--check)
                CHECK_ONLY=true
                shift
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -f|--force)
                FORCE=true
                shift
                ;;
            -*)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
            *)
                if [ -z "$VERSION" ]; then
                    VERSION=$1
                else
                    print_error "Multiple versions specified: $VERSION and $1"
                    exit 1
                fi
                shift
                ;;
        esac
    done
    
    # Check if we're in a git repository
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        print_error "Not in a git repository"
        exit 1
    fi
    
    # Check if we're on main branch
    local current_branch=$(git branch --show-current)
    if [ "$current_branch" != "main" ]; then
        print_warning "Not on main branch (currently on: $current_branch)"
        if [ "$FORCE" != "true" ]; then
            print_error "Please switch to main branch before releasing"
            print_error "Use --force to override this check"
            exit 1
        fi
    fi
    
    # Get current version
    local current_version=$(get_current_version)
    if [ -z "$current_version" ]; then
        print_error "Could not determine current version from Cargo.toml"
        exit 1
    fi
    
    print_status "Current version: $current_version"
    
    # If only checking, show suggestions and exit
    if [ "$CHECK_ONLY" = "true" ]; then
        echo
        print_status "Suggested next versions:"
        echo "  Patch: $(increment_version "$current_version" "patch")"
        echo "  Minor: $(increment_version "$current_version" "minor")"
        echo "  Major: $(increment_version "$current_version" "major")"
        exit 0
    fi
    
    # Determine new version
    if [ -z "$VERSION" ]; then
        if [ -z "$INCREMENT_TYPE" ]; then
            print_error "Please specify a version or increment type"
            show_usage
            exit 1
        fi
        VERSION=$(increment_version "$current_version" "$INCREMENT_TYPE")
        print_status "Incrementing $INCREMENT_TYPE version: $current_version -> $VERSION"
    else
        # Validate version format
        if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            print_error "Invalid version format: $VERSION (expected: X.Y.Z)"
            exit 1
        fi
        
        if [ "$VERSION" = "$current_version" ]; then
            print_warning "Version is already $VERSION"
            if [ "$FORCE" != "true" ]; then
                print_error "Use --force to proceed anyway"
                exit 1
            fi
        fi
    fi
    
    print_status "New version: $VERSION"
    
    # Check working directory
    check_working_directory
    
    # Update version
    update_version "$VERSION"
    
    # Create git tag
    create_git_tag "$VERSION"
    
    # Push changes
    push_changes "$VERSION"
    
    # Show summary
    show_release_summary "$VERSION"
    
    if [ "$DRY_RUN" = "true" ]; then
        print_warning "This was a dry run. No changes were made."
    fi
}

# Run main function with all arguments
main "$@" 