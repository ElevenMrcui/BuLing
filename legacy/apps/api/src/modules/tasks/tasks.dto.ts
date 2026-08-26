import { IsArray, IsIn, IsOptional, IsString, MaxLength, MinLength } from "class-validator";
import type { AssignMode, Priority, PublishTaskInput, ClaimTaskInput } from "@buling/shared";

export class PublishTaskDto implements PublishTaskInput {
  @IsString() @MinLength(1) @MaxLength(200)
  title!: string;

  @IsOptional() @IsString() @MaxLength(4000)
  description?: string;

  @IsArray() @IsString({ each: true })
  requiredSkills!: string[];

  @IsOptional() @IsIn(["high", "mid", "low"])
  priority?: Priority;

  @IsOptional() @IsString()
  due?: string;

  @IsIn(["auto", "manual", "team"])
  assignMode!: AssignMode;

  /** assignMode='manual' 时必填 */
  @IsOptional() @IsString()
  assigneeAgentId?: string;

  /** assignMode='team' 时必填 */
  @IsOptional() @IsString()
  teamId?: string;

  /** assignMode='team' 时必填；不填默认取 team.ownerAgentId */
  @IsOptional() @IsString()
  teamOwnerAgentId?: string;
}

export class ClaimTaskDto implements ClaimTaskInput {
  @IsString()
  agentId!: string;
}
