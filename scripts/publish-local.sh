#!/bin/bash

# Local publishing script that mimics the GitHub Action workflow
# This script publishes packages to NPM using the token from .env file

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Check if we're in the right directory
if [ ! -f "package.json" ] || [ ! -d "packages" ]; then
    print_error "Run this script from the repository root"
    exit 1
fi

# Load environment variables
if [ ! -f ".env" ]; then
    print_error ".env file not found. Please create it with NPM_TOKEN=your_token"
    exit 1
fi

source .env

if [ -z "$NPM_TOKEN" ]; then
    print_error "NPM_TOKEN not found in .env file"
    exit 1
fi

# Parse command line arguments
VERSION_TYPE="patch"
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --version-type)
            VERSION_TYPE="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            echo "Usage: $0 [--version-type patch|minor|major|prerelease] [--dry-run]"
            echo ""
            echo "Options:"
            echo "  --version-type    Version bump type (default: patch)"
            echo "  --dry-run        Don't actually publish, just show what would happen"
            echo "  --help           Show this help message"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

print_step "Local NPM Publishing Script"
echo "Version type: $VERSION_TYPE"
echo "Dry run: $DRY_RUN"
echo ""

if [ "$DRY_RUN" = true ]; then
    print_warning "DRY RUN MODE - No actual publishing will occur"
    echo ""
fi

# Step 1: Install dependencies
print_step "Installing dependencies"
npm install
print_success "Dependencies installed"
echo ""

# Step 2: Build packages
print_step "Building packages"
npm run build
print_success "Packages built"
echo ""

# Step 3: Run tests
print_step "Running tests"
npm test
print_success "Tests passed"
echo ""

# Step 4: Configure NPM authentication
print_step "Configuring NPM authentication"
echo "//registry.npmjs.org/:_authToken=$NPM_TOKEN" > ~/.npmrc
print_success "NPM authentication configured"
echo ""

# Function to bump version in package.json
bump_version() {
    local package_dir=$1
    local version_type=$2
    
    cd "$package_dir"
    if [ "$DRY_RUN" = true ]; then
        local current_version=$(node -p "require('./package.json').version")
        echo "Would bump $package_dir from $current_version ($version_type)" >&2
        echo $current_version
    else
        npm version $version_type --no-git-tag-version >&2
        local new_version=$(node -p "require('./package.json').version")
        echo "Bumped $package_dir to $new_version" >&2
        echo $new_version
    fi
    cd - > /dev/null
}

# Function to update dependency versions
update_dependency() {
    local package_file=$1
    local dep_name=$2
    local new_version=$3
    
    if [ "$DRY_RUN" = true ]; then
        echo "Would update $dep_name to ^$new_version in $package_file"
        return
    fi
    
    # Update dependency version using node
    node -e "
        const fs = require('fs');
        const pkg = JSON.parse(fs.readFileSync('$package_file', 'utf8'));
        if (pkg.dependencies && pkg.dependencies['$dep_name']) {
            pkg.dependencies['$dep_name'] = '^$new_version';
        }
        if (pkg.devDependencies && pkg.devDependencies['$dep_name']) {
            pkg.devDependencies['$dep_name'] = '^$new_version';
        }
        fs.writeFileSync('$package_file', JSON.stringify(pkg, null, 2) + '\n');
    "
}

# Step 5: Version bumping and syncing
print_step "Version bumping and syncing"

workspace_version=$(node -p "require('./package.json').version")
echo "Current workspace version: $workspace_version"

# Function to set package version to a specific version
set_package_version() {
    local package_file=$1
    local package_name=$2
    local target_version=$3
    
    if [ "$DRY_RUN" = true ]; then
        if [ -f "$package_file" ]; then
            local current_version=$(node -p "require('./$package_file').version")
            echo "Would set $package_name from $current_version to $target_version"
        else
            echo "Would set $package_name (file not found: $package_file) to $target_version"
        fi
        return
    fi
    
    # Update package version to target version
    if [ -f "$package_file" ]; then
        node -e "
            const fs = require('fs');
            const pkg = JSON.parse(fs.readFileSync('$package_file', 'utf8'));
            const oldVersion = pkg.version;
            pkg.version = '$target_version';
            fs.writeFileSync('$package_file', JSON.stringify(pkg, null, 2) + '\n');
            console.log('Set $package_name from ' + oldVersion + ' to $target_version');
        "
    else
        echo "Warning: Package file not found: $package_file"
    fi
}

# Calculate new version by bumping workspace version
if [ "$DRY_RUN" = true ]; then
    echo "Would bump workspace from $workspace_version ($VERSION_TYPE)"
    # Simulate version bump for dry run
    case $VERSION_TYPE in
        "patch")
            new_workspace_version=$(node -e "const semver = require('semver'); console.log(semver.inc('$workspace_version', 'patch'))")
            ;;
        "minor")
            new_workspace_version=$(node -e "const semver = require('semver'); console.log(semver.inc('$workspace_version', 'minor'))")
            ;;
        "major")
            new_workspace_version=$(node -e "const semver = require('semver'); console.log(semver.inc('$workspace_version', 'major'))")
            ;;
        *)
            new_workspace_version=$workspace_version
            ;;
    esac
else
    npm version $VERSION_TYPE --no-git-tag-version
    new_workspace_version=$(node -p "require('./package.json').version")
    echo "Bumped workspace to $new_workspace_version"
fi

echo "Target version for all packages: $new_workspace_version"
echo ""

# Set all packages to the new version
echo "Setting all packages to version $new_workspace_version..."
set_package_version "packages/core/package.json" "@uepm/core" "$new_workspace_version"
set_package_version "packages/init/package.json" "@uepm/init" "$new_workspace_version"
set_package_version "packages/postinstall/package.json" "@uepm/postinstall" "$new_workspace_version"
set_package_version "samples/plugins/example-plugin/package.json" "@uepm/example-plugin" "$new_workspace_version"
set_package_version "samples/plugins/dependency-plugin/package.json" "@uepm/dependency-plugin" "$new_workspace_version"

# Update internal dependencies to use new version
echo ""
echo "Updating internal dependencies to version $new_workspace_version..."
update_dependency "packages/init/package.json" "@uepm/core" "$new_workspace_version"
update_dependency "packages/postinstall/package.json" "@uepm/core" "$new_workspace_version"
update_dependency "samples/plugins/dependency-plugin/package.json" "@uepm/example-plugin" "$new_workspace_version"
update_dependency "samples/project/package.json" "@uepm/postinstall" "$new_workspace_version"

# Set final version variables for publishing
core_version=$new_workspace_version
init_version=$new_workspace_version
postinstall_version=$new_workspace_version
example_version=$new_workspace_version
dependency_version=$new_workspace_version

print_success "Version bumping complete"
echo ""

if [ "$DRY_RUN" = false ]; then
    # Step 7: Rebuild after version updates
    print_step "Rebuilding packages after version updates"
    npm run build
    print_success "Packages rebuilt"
    echo ""

    # Step 8: Publishing
    print_step "Publishing packages to NPM"

    REPO_ROOT="$(pwd)"

    echo "Publishing @uepm/core@$core_version"
    npm publish --access public --workspace=packages/core
    print_success "Published @uepm/core@$core_version"

    echo "Publishing @uepm/init@$init_version"
    npm publish --access public --workspace=packages/init
    print_success "Published @uepm/init@$init_version"

    echo "Publishing @uepm/postinstall@$postinstall_version"
    npm publish --access public --workspace=packages/postinstall
    print_success "Published @uepm/postinstall@$postinstall_version"

    echo "Publishing @uepm/example-plugin@$example_version"
    npm publish --access public --workspace=samples/plugins/example-plugin
    print_success "Published @uepm/example-plugin@$example_version"

    echo "Publishing @uepm/dependency-plugin@$dependency_version"
    npm publish --access public --workspace=samples/plugins/dependency-plugin
    print_success "Published @uepm/dependency-plugin@$dependency_version"

    echo ""
    print_step "Publication Summary"
    echo "@uepm/core: $core_version"
    echo "@uepm/init: $init_version"
    echo "@uepm/postinstall: $postinstall_version"
    echo "@uepm/example-plugin: $example_version"
    echo "@uepm/dependency-plugin: $dependency_version"
    echo ""

    print_success "All packages published successfully!"
    echo ""

    # Step 9: Git tag and GitHub release
    print_step "Creating git tag and GitHub release"

    TAG="v$new_workspace_version"

    git tag "$TAG"
    git push origin "$TAG"
    print_success "Pushed tag $TAG"

    gh release create "$TAG" \
        --title "UEPM $TAG" \
        --generate-notes \
        --latest
    print_success "GitHub release $TAG created"

    echo ""
    echo "You can now test the published packages:"
    echo "  npx @uepm/init@$init_version"
    echo "  npm install @uepm/example-plugin@$example_version"

else
    echo ""
    print_warning "DRY RUN COMPLETE - No packages were actually published"
    echo ""
    echo "Would also create git tag v$new_workspace_version and GitHub release."
    echo ""
    echo "To publish for real, run:"
    echo "  $0 --version-type $VERSION_TYPE"
fi

# Clean up NPM config
rm -f ~/.npmrc
print_success "Cleaned up NPM authentication"